use std::{
    collections::HashMap,
    net::SocketAddrV4,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, MutexGuard,
    },
};

use anyhow::Error;
use flate2::Compression;
use threadpool::ThreadPool;

use crate::{
    chunk::Chunk,
    generation::{
        ChunkGenerator, DynChunkGenerator, DynGeneratorFactory, FactoryParameters, GenerationArgs,
    },
};

use super::net::{
    packets::{self, ReplyChunk},
    AddressedPacket, Networker,
};

// TODO: We use a lot of request ID, client ID, generator ID, etc. Currently these are all just u32s which makes it hard to tell which is which,
// so we should have separate types for separate IDs. This could also allow RequestIdent to be .into()'ed into these other types for extra ergonomics!
#[derive(Clone, Copy, Debug)]
struct RequestIdent {
    request_id: u32,
    client_id: u32,
}

impl RequestIdent {
    fn new(request_id: u32, client_id: u32) -> Self {
        Self {
            request_id,
            client_id,
        }
    }
}

enum GenerationResult {
    Success(RequestIdent, Chunk),
    Failure(RequestIdent, Error),
}

impl GenerationResult {
    fn id(&self) -> RequestIdent {
        match self {
            Self::Success(id, _) => *id,
            Self::Failure(id, _) => *id,
        }
    }

    fn from_result(res: anyhow::Result<Chunk>, id: RequestIdent) -> Self {
        match res {
            anyhow::Result::Ok(chunk) => Self::Success(id, chunk),
            anyhow::Result::Err(error) => Self::Failure(id, error),
        }
    }
}

struct CompletedChunksIterator<'a> {
    rx: MutexGuard<'a, Receiver<GenerationResult>>,
}

impl<'a> Iterator for CompletedChunksIterator<'a> {
    type Item = GenerationResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.try_recv().ok()
    }
}

#[derive(Debug, te::Error)]
#[error("Generator not found with ID {0}")]
struct ManagerSubmitError(u32);

#[derive(Debug, te::Error)]
#[error("Couldn't find generator factory with name '{0}'")]
struct UnknownFactoryError<'a>(&'a str);

#[derive(Clone)]
struct ChunkReceiver {
    rx: Arc<Mutex<Receiver<GenerationResult>>>,
}

impl ChunkReceiver {
    fn completed(&self) -> CompletedChunksIterator<'_> {
        CompletedChunksIterator {
            rx: self.rx.lock().unwrap(),
        }
    }
}

struct GeneratorManager {
    factories: HashMap<&'static str, Box<dyn DynGeneratorFactory>>,
    instances: HashMap<u32, Arc<Box<dyn DynChunkGenerator>>>,
    workers: Mutex<ThreadPool>,
    channel_pair: (
        Sender<GenerationResult>,
        Arc<Mutex<Receiver<GenerationResult>>>,
    ),
}

impl GeneratorManager {
    fn new() -> Self {
        Self {
            factories: HashMap::new(),
            instances: HashMap::new(),
            workers: Mutex::new(Default::default()),
            channel_pair: {
                let (tx, rx) = mpsc::channel::<GenerationResult>();
                (tx, Arc::new(Mutex::new(rx)))
            },
        }
    }

    fn receiver(&self) -> ChunkReceiver {
        ChunkReceiver {
            rx: self.channel_pair.1.clone(),
        }
    }

    fn create_gen_instance<'a>(
        &self,
        generator_name: &'a str,
        factory_params: FactoryParameters<'_>,
    ) -> Result<Box<dyn DynChunkGenerator>, UnknownFactoryError<'a>> {
        self.factories
            .get(generator_name)
            .map(|f| f.create(factory_params))
            .ok_or(UnknownFactoryError(generator_name))
    }

    fn random_gen_id(&self) -> u32 {
        loop {
            let id = rand::random::<u32>();
            if !self.instances.contains_key(&id) {
                return id;
            }
        }
    }

    pub fn register_generator<'a>(
        &mut self,
        generator_name: &'a str,
        factory_params: FactoryParameters<'_>,
    ) -> Result<u32, UnknownFactoryError<'a>> {
        let instance = self.create_gen_instance(generator_name, factory_params)?;
        let id = self.random_gen_id();

        self.instances.insert(id, Arc::new(instance));
        Ok(id)
    }

    pub fn submit_chunk(
        &self,
        request_ident: RequestIdent,
        generator_id: u32,
        args: GenerationArgs,
    ) -> Result<(), ManagerSubmitError> {
        let tx = self.channel_pair.0.clone();
        let instance = self
            .instances
            .get(&generator_id)
            .ok_or(ManagerSubmitError(generator_id))?
            .clone();

        self.workers.lock().unwrap().execute(move || {
            let result = GenerationResult::from_result(instance.generate(&args), request_ident);
            tx.send(result).unwrap();
        });

        Ok(())
    }

    pub fn completed(&self) -> CompletedChunksIterator<'_> {
        CompletedChunksIterator {
            rx: self.channel_pair.1.lock().unwrap(),
        }
    }

    pub fn add_factory(&mut self, name: &'static str, factory: Box<dyn DynGeneratorFactory>) {
        self.factories.insert(name, factory);
    }
}

pub struct ServerParams {
    pub(crate) addr: SocketAddrV4,
    pub(crate) compression: Compression,
    pub(crate) coarsening: u32,
}

pub struct Server {
    net: Networker,
    generators: Arc<Mutex<GeneratorManager>>,
    params: ServerParams,
    running: Arc<AtomicBool>,
}

impl Server {
    pub fn new(params: ServerParams) -> Self {
        Self {
            net: Networker::new(),
            generators: Mutex::new(GeneratorManager::new()).into(),
            params,
            running: Arc::new(AtomicBool::from(false)),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst)
    }

    pub fn add_generator<G: ChunkGenerator>(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Cannot add new generator while server is running!");
        }

        self.generators
            .lock()
            .unwrap()
            .add_factory(G::NAME, Box::new(G::factory()));
    }

    /// Start the client request handler thread. This thread handles requests from clients such as
    /// submitting chunks for generation and registering new chunk generators with provided parameters.
    fn start_client_request_handler(&self) {
        let coarsening = self.params.coarsening;

        let running = self.running.clone();
        let net = self.net.clone();
        let manager = self.generators.clone();

        // This thread submits chunks for generation and registers generators at the request of clients.
        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                // Coarsen the atomic access so the loop can be faster.
                for _ in 0..coarsening {
                    for incoming in net.incoming() {
                        if let Some(packet) = incoming.downcast_ref::<packets::GenerateChunk>() {
                            let request_ident = RequestIdent::new(packet.request_id, incoming.id());

                            if let Err(error) = manager.lock().unwrap().submit_chunk(
                                request_ident,
                                packet.generator_id,
                                packet.args(),
                            ) {
                                log::error!("Request {request_ident:?} failed when submitting chunk for generation: {error}");
                            }
                        }

                        if let Some(packet) = incoming.downcast_ref::<packets::AddGenerator>() {
                            let request_ident = RequestIdent::new(packet.request_id, incoming.id());

                            if let Ok(generator_id) = manager
                                .lock()
                                .unwrap()
                                .register_generator(&packet.name, packet.factory_params())
                            {
                                net.send(AddressedPacket::new(
                                    request_ident.client_id,
                                    packets::ConfirmGeneratorAddition::new(
                                        request_ident.request_id,
                                        generator_id,
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        });
    }

    /// Start the distributor thread for generated chunks. This thread collects chunks from the generator pool and
    /// sends them to their respective clients.
    fn start_chunk_distributor(&self) {
        let coarsening = self.params.coarsening;

        let receiver = self.generators.lock().unwrap().receiver();
        let net = self.net.clone();
        let running = self.running.clone();

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                for _ in 0..coarsening {
                    for completed in receiver.completed() {
                        match completed {
                            GenerationResult::Success(ident, chunk) => {
                                let packet = ReplyChunk {
                                    request_id: ident.request_id,
                                    chunk,
                                };
                                net.send(AddressedPacket::new(ident.client_id, packet));
                            }
                            GenerationResult::Failure(ident, error) => {
                                log::error!("Request {ident:?} failed: {error}");
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn run(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Server is already running")
        }

        self.running.store(true, Ordering::SeqCst);

        self.net.run(super::net::Params {
            addr: self.params.addr,
            compression: self.params.compression,
        });

        self.start_client_request_handler();
        self.start_chunk_distributor();

        // todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };

    use flate2::{read::ZlibDecoder, write::ZlibEncoder};
    use volume::Volume;

    use crate::{
        block::BlockId,
        chunk::Spaces,
        generation::GeneratorFactory,
        runtime::net::{internal::Header, packets::Packet},
    };

    use super::*;

    struct MockGenFactory;

    impl GeneratorFactory for MockGenFactory {
        type Generator = MockGenerator;

        fn create(&self, params: FactoryParameters<'_>) -> Self::Generator {
            MockGenerator {
                min_height: params.min_height,
                max_height: params.max_height,
                default_id: params.default,
            }
        }
    }

    struct MockGenerator {
        min_height: i32,
        max_height: i32,
        default_id: BlockId,
    }

    impl ChunkGenerator for MockGenerator {
        const NAME: &'static str = "MOCK_GENERATOR";

        type Factory = MockGenFactory;

        fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk> {
            let mut chunk = Chunk::new(self.default_id, args.pos, self.min_height, self.max_height);

            for x in 0..16 {
                for z in 0..16 {
                    chunk.set(Spaces::Cs([x, self.min_height, z]), 80.into());
                }
            }

            Ok(chunk)
        }

        fn factory() -> Self::Factory {
            MockGenFactory
        }
    }

    struct MockClient {
        stream: TcpStream,
    }

    impl MockClient {
        fn new(addr: SocketAddrV4) -> Self {
            Self {
                stream: TcpStream::connect(addr).unwrap(),
            }
        }

        fn send_packet<P: Packet>(&mut self, packet: &P) -> anyhow::Result<()> {
            let mut buf = P::ID.to_be_bytes().to_vec();
            buf.extend(bincode::serialize(packet)?);

            let decompressed_len = buf.len();

            let compressed_buf = {
                let mut compressed_buf = Vec::<u8>::new();
                let mut compressor = ZlibEncoder::new(&mut compressed_buf, Compression::best());
                compressor.write_all(&buf)?;
                compressor.finish()?;

                compressed_buf
            };

            let header = Header::new(compressed_buf.len() as u32, decompressed_len as u32);

            header.sync_write(&mut self.stream)?;
            self.stream.write_all(&compressed_buf)?;

            Ok(())
        }

        fn read_packet<P: Packet>(&mut self) -> anyhow::Result<P> {
            let header = Header::sync_read(&mut self.stream)?;
            let mut compressed_buf = vec![0u8; header.compressed_len as usize];

            self.stream.read_exact(&mut compressed_buf)?;

            let decompressed_buf = {
                let mut buf = Vec::<u8>::with_capacity(header.decompressed_len as usize);
                let mut decompressor = ZlibDecoder::new(&compressed_buf[..]);

                decompressor.read_to_end(&mut buf)?;
                buf
            };

            let packet = bincode::deserialize::<P>(&decompressed_buf[2..])?;
            Ok(packet)
        }
    }

    #[test]
    fn end_to_end_server_test() {
        let params = ServerParams {
            addr: "0.0.0.0:33443".parse().unwrap(),
            compression: Compression::best(),
            coarsening: 100,
        };

        let mut server = Server::new(params);

        server.add_generator::<MockGenerator>();

        server.run();

        let mut client = MockClient::new("127.0.0.1:33443".parse().unwrap());

        client
            .send_packet(&packets::AddGenerator {
                request_id: 500,
                name: MockGenerator::NAME.to_string(),
                min_height: -64,
                max_height: 320,
                default_id: 21.into(),
            })
            .unwrap();

        let generator_id = {
            let packet = client
                .read_packet::<packets::ConfirmGeneratorAddition>()
                .unwrap();
            assert_eq!(packet.request_id, 500);
            packet.generator_id
        };

        client
            .send_packet(&packets::GenerateChunk {
                request_id: 420,
                generator_id,
                pos: na::vector![6i32, 4],
            })
            .unwrap();

        let packet = client.read_packet::<packets::ReplyChunk>().unwrap();
        assert_eq!(packet.request_id, 420);

        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(
                    packet.chunk.get(Spaces::Cs([x, -64, z])),
                    Some(&BlockId::new(80))
                );
            }
        }
    }
}
