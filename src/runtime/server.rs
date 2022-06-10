use std::{
    collections::HashMap,
    net::SocketAddrV4,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, MutexGuard, RwLock,
    },
};

use anyhow::Error;
use flate2::Compression;
use threadpool::ThreadPool;

use crate::{
    chunk::Chunk,
    generation::{DynChunkGenerator, FactoryParameters, GenerationArgs, GeneratorFactory},
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
    factories: HashMap<&'static str, Box<dyn GeneratorFactory>>,
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
}

pub struct ServerParams {
    addr: SocketAddrV4,
    compression: Compression,
    coarsening: u32,
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

    pub fn run(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Server is already running")
        }

        self.running.store(true, Ordering::SeqCst);

        self.net.run(super::net::Params {
            addr: self.params.addr,
            compression: self.params.compression,
        });

        let running = self.running.clone();
        let coarsening = self.params.coarsening;
        let net = self.net.clone();
        let manager = self.generators.clone();

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

        todo!()
    }
}
