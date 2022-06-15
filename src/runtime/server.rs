use std::{
    collections::HashMap,
    net::SocketAddrV4,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Error;
use flate2::Compression;
use threadpool::ThreadPool;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

use crate::{
    chunk::Chunk,
    generation::{
        ChunkGenerator, DynChunkGenerator, DynGeneratorFactory, FactoryParameters, GenerationArgs,
    },
};

use super::{
    net::{
        packets::{self, ProtocolError, ProtocolErrorKind, ReplyChunk},
        Networker,
    },
    util::{GenerationIdent, GeneratorId, RequestIdent},
};

#[derive(Debug)]
enum GenerationResult {
    Success(GenerationIdent, Chunk),
    Failure(GenerationIdent, Error),
}

impl GenerationResult {
    fn ident(&self) -> GenerationIdent {
        match self {
            Self::Success(id, _) => *id,
            Self::Failure(id, _) => *id,
        }
    }

    fn from_result(res: anyhow::Result<Chunk>, id: GenerationIdent) -> Self {
        match res {
            anyhow::Result::Ok(chunk) => Self::Success(id, chunk),
            anyhow::Result::Err(error) => Self::Failure(id, error),
        }
    }
}

struct CompletedChunksIterator(std::vec::IntoIter<GenerationResult>);

impl Iterator for CompletedChunksIterator {
    type Item = GenerationResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[derive(Debug, te::Error)]
#[error("Generator not found with ID {0}")]
struct ManagerSubmitError(GeneratorId);

#[derive(Debug, te::Error)]
#[error("Couldn't find generator factory with name '{0}'")]
struct UnknownFactoryError<'a>(&'a str);

#[derive(Clone)]
struct ChunkReceiver {
    rx: Arc<Mutex<Receiver<GenerationResult>>>,
}

impl ChunkReceiver {
    async fn completed(&self) -> CompletedChunksIterator {
        let mut chunks = Vec::new();
        let mut guard = self.rx.lock().await;

        while let Ok(chunk_result) = guard.try_recv() {
            chunks.push(chunk_result);
        }

        CompletedChunksIterator(chunks.into_iter())
    }
}

struct GeneratorManager {
    factories: HashMap<&'static str, Box<dyn DynGeneratorFactory>>,
    instances: HashMap<GeneratorId, Arc<Box<dyn DynChunkGenerator>>>,
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
                let (tx, rx) = mpsc::channel::<GenerationResult>(128);
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

    fn random_gen_id(&self) -> GeneratorId {
        loop {
            let id = rand::random::<u32>();
            if !self.instances.contains_key(&id.into()) {
                return id.into();
            }
        }
    }

    pub fn register_generator<'a>(
        &mut self,
        generator_name: &'a str,
        factory_params: FactoryParameters<'_>,
    ) -> Result<GeneratorId, UnknownFactoryError<'a>> {
        let instance = self.create_gen_instance(generator_name, factory_params)?;
        let id = self.random_gen_id();

        self.instances.insert(id, Arc::new(instance));
        Ok(id)
    }

    pub async fn submit_chunk(
        &self,
        request_ident: RequestIdent,
        generator_id: GeneratorId,
        args: GenerationArgs,
    ) -> Result<(), ManagerSubmitError> {
        let tx = self.channel_pair.0.clone();
        let instance = self
            .instances
            .get(&generator_id)
            .ok_or(ManagerSubmitError(generator_id))?
            .clone();

        self.workers.lock().await.execute(move || {
            let result = GenerationResult::from_result(
                instance.generate(&args),
                request_ident.generation_ident(generator_id),
            );
            tx.blocking_send(result).unwrap();
        });

        Ok(())
    }

    pub async fn completed(&self) -> CompletedChunksIterator {
        let mut chunks = Vec::new();
        let mut guard = self.channel_pair.1.lock().await;

        while let Some(chunk_result) = guard.recv().await {
            chunks.push(chunk_result);
        }

        CompletedChunksIterator(chunks.into_iter())
    }

    pub fn add_factory(&mut self, name: &'static str, factory: Box<dyn DynGeneratorFactory>) {
        self.factories.insert(name, factory);
    }
}

#[derive(Copy, Clone)]
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
            net: Networker::new(params.into()),
            generators: Mutex::new(GeneratorManager::new()).into(),
            params,
            running: Arc::new(AtomicBool::from(false)),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst)
    }

    pub async fn add_generator<G: ChunkGenerator>(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Cannot add new generator while server is running!");
        }

        self.generators
            .lock()
            .await
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
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                // Coarsen the atomic access so the loop can be faster.
                for _ in 0..coarsening {
                    for (conn, packet) in net.incoming().await {
                        match packet {
                            Ok(packet) => {
                                if let Some(packet) =
                                    packet.downcast_ref::<packets::GenerateChunk>()
                                {
                                    let request_ident =
                                        RequestIdent::new(packet.request_id, conn.id());

                                    {
                                        if let Err(error) = manager
                                            .lock()
                                            .await
                                            .submit_chunk(
                                                request_ident,
                                                packet.generator_id,
                                                packet.args(),
                                            )
                                            .await
                                        {
                                            log::error!("Request {request_ident:?} failed when submitting chunk for generation: {error}");
                                        }
                                    }
                                }

                                if let Some(packet) = packet.downcast_ref::<packets::AddGenerator>()
                                {
                                    let request_ident =
                                        RequestIdent::new(packet.request_id, conn.id());

                                    if let Ok(generator_id) = manager
                                        .lock()
                                        .await
                                        .register_generator(&packet.name, packet.factory_params())
                                    {
                                        conn.send_packet(&packets::ConfirmGeneratorAddition::new(
                                            request_ident.request_id,
                                            generator_id,
                                        ))
                                        .await
                                        .unwrap();
                                    }
                                }
                            }
                            Err(error) => {
                                conn.send_packet(&ProtocolError::fatal(ProtocolErrorKind::Other {
                                    details: error.to_string(),
                                }))
                                .await
                                .unwrap();
                            }
                        }
                    }
                }
            }
        });
    }

    /// Start the distributor thread for generated chunks. This thread collects chunks from the generator pool and
    /// sends them to their respective clients.
    async fn start_chunk_distributor(&self) {
        let coarsening = self.params.coarsening;

        let receiver = self.generators.lock().await.receiver();
        let net = self.net.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                for _ in 0..coarsening {
                    for completed in receiver.completed().await.collect::<Vec<_>>().into_iter() {
                        match completed {
                            GenerationResult::Success(ident, chunk) => {
                                let packet = ReplyChunk {
                                    request_id: ident.into(),
                                    chunk,
                                };

                                if let Some(conn) = net.connection(ident.into()).await {
                                    conn.send_packet(&packet).await.unwrap();
                                }
                            }
                            GenerationResult::Failure(ident, error) => {
                                log::error!("Request {ident:?} failed: {error}");
                                // let net_error = ProtocolErrorKind::ChunkGenerationFailure { generator_id: , request_id: () };
                                // let packet = ProtocolError::gentle()
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn run(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Server is already running")
        }

        self.running.store(true, Ordering::SeqCst);

        self.net.run().await.unwrap();

        self.start_client_request_handler();
        self.start_chunk_distributor().await;
    }
}
