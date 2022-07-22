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

use crate::generation::{ChunkGenerator, DynChunkGenerator, DynGeneratorFactory};
use common::generation::{FactoryParameters, GenerationArgs};

use super::{
    dispatcher::Dispatcher,
    events::{self, ChunkFinished, Context, IncomingPacket},
    net::Networker,
    util::{GenerationIdent, RequestIdent},
};

use common::{packets, Chunk, GeneratorId};

#[derive(Debug)]
pub enum GenerationResult {
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

pub struct CompletedChunksIterator(std::vec::IntoIter<GenerationResult>);

impl Iterator for CompletedChunksIterator {
    type Item = GenerationResult;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[derive(Debug, te::Error)]
#[error("Generator not found with ID {0}")]
pub struct ManagerSubmitError(GeneratorId);

#[derive(Debug, te::Error)]
#[error("Couldn't find generator factory with name '{0}'")]
pub struct UnknownFactoryError<'a>(&'a str);

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

pub struct GeneratorManager {
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
    dispatcher: Arc<Dispatcher<Context>>,
}

impl Server {
    pub async fn new(params: ServerParams) -> Self {
        let dispatcher = Dispatcher::new(20);
        events::defaults(&dispatcher).await;

        Self {
            net: Networker::new(params.into()),
            generators: Mutex::new(GeneratorManager::new()).into(),
            params,
            running: Arc::new(AtomicBool::from(false)),
            dispatcher: Arc::new(dispatcher),
        }
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.net.stop().await
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
        let dispatcher = self.dispatcher.clone();

        // This thread submits chunks for generation and registers generators at the request of clients.
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                // Coarsen the atomic access so the loop can be faster.
                for _ in 0..coarsening {
                    for (conn, packet) in net.incoming().await {
                        match packet {
                            Ok(packet) => {
                                let event = IncomingPacket {
                                    connection: conn,
                                    packet: packet.into(),
                                };

                                dispatcher
                                    .fire_event(
                                        Context {
                                            dispatcher: dispatcher.clone(),
                                            generators: manager.clone(),
                                            networker: net.clone(),
                                        },
                                        event,
                                    )
                                    .await;
                            }
                            Err(error) => {
                                conn.send_packet(&packets::ProtocolError::fatal(
                                    packets::ProtocolErrorKind::Other {
                                        details: error.to_string(),
                                    },
                                ))
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
    fn start_chunk_distributor(&self) {
        let coarsening = self.params.coarsening;

        // We don't need this function to be async, doing so would just add needless complexity, so we access the async mutex by blocking.
        let receiver = tokio::task::block_in_place(|| self.generators.blocking_lock().receiver());
        let generators = self.generators.clone();
        let net = self.net.clone();
        let running = self.running.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                for _ in 0..coarsening {
                    for completed in receiver.completed().await.collect::<Vec<_>>().into_iter() {
                        let ctx = Context {
                            dispatcher: dispatcher.clone(),
                            generators: generators.clone(),
                            networker: net.clone(),
                        };
                        let event = ChunkFinished {
                            result: Arc::new(completed),
                        };

                        dispatcher.fire_event(ctx, event).await;
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

        log::info!("Starting internal networker...");
        self.net.run().await.unwrap();

        log::info!("Starting client request handler...");
        self.start_client_request_handler();
        log::info!("Starting chunk distributor...");
        self.start_chunk_distributor();
    }
}
