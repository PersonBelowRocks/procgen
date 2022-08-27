use std::{
    net::SocketAddrV4,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use flate2::Compression;
use tokio::sync::Mutex;

use crate::generation::{BrushGeneratorFactory, GeneratorManager, RegionGeneratorFactory};

use super::{
    dispatcher::Dispatcher,
    events::{self, Context},
    net::{Connection, ConnectionRegistry, Listener},
};

#[derive(Copy, Clone)]
pub struct ServerParams {
    pub(crate) addr: SocketAddrV4,
    pub(crate) compression: Compression,
    pub(crate) coarsening: u32,
}

pub struct Server {
    registry: Arc<ConnectionRegistry>,
    listener: Arc<Listener>,

    generators: Arc<Mutex<GeneratorManager>>,
    params: ServerParams,
    running: Arc<AtomicBool>,
    dispatcher: Arc<Dispatcher<Context>>,
}

impl Server {
    pub async fn new(params: ServerParams) -> Self {
        let dispatcher = Arc::new(Dispatcher::new(20));
        let manager = GeneratorManager::new();

        events::defaults(dispatcher.as_ref(), &manager).await;

        Self {
            registry: Arc::new(ConnectionRegistry::new()),
            listener: Arc::new(Listener::new(dispatcher.clone())),

            generators: Mutex::new(manager).into(),
            params,
            running: Arc::new(AtomicBool::from(false)),
            dispatcher,
        }
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        log::info!("Stopping server...");

        self.running.store(false, Ordering::SeqCst);
        self.listener.stop();
        self.registry.disconnect_all().await?;

        Ok(())
    }

    pub async fn add_region_generator<Fact: RegionGeneratorFactory>(&self, factory: Fact) {
        self.generators
            .lock()
            .await
            .add_region_factory(factory)
            .await;
    }

    pub async fn add_brush_generator<Fact: BrushGeneratorFactory>(&self, factory: Fact) {
        self.generators
            .lock()
            .await
            .add_brush_factory(factory)
            .await;
    }

    fn start_listener(&self) {
        let listener = self.listener.clone();

        let address = self.params.addr;
        let compression = self.params.compression;

        let connections = self.registry.clone();
        let dispatcher = self.dispatcher.clone();

        let context = Context {
            dispatcher: dispatcher.clone(),
            generators: self.generators.clone(),
            connections: connections.clone(),
        };

        tokio::spawn(async move {
            if let Err(error) = listener.start(address).await {
                log::error!("Unable to connect to address {address}: {error}");
                return;
            }

            while listener.running() {
                match listener.accept().await {
                    Ok((stream, address)) => {
                        log::info!("Accepted connection from {address}");
                        let connection = Connection::start(
                            stream,
                            compression,
                            dispatcher.clone(),
                            context.clone(),
                        );
                        connections.add_connection(connection).await;
                    }
                    Err(error) => {
                        log::error!(
                            "Error accepting incoming connection: {error}, stopping listener..."
                        );
                        listener.stop();
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

        log::info!("Starting listener...");
        self.start_listener();
    }
}
