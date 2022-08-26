use std::{
    net::SocketAddrV4,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use flate2::Compression;
use procgen_common::packets;
use tokio::sync::Mutex;

use crate::generation::{BrushGeneratorFactory, GeneratorManager, RegionGeneratorFactory};

use super::{
    dispatcher::Dispatcher,
    events::{self, Context, IncomingPacket},
    net::Networker,
};

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
        let manager = GeneratorManager::new();

        events::defaults(&dispatcher, &manager).await;

        Self {
            net: Networker::new(params.into()),
            generators: Mutex::new(manager).into(),
            params,
            running: Arc::new(AtomicBool::from(false)),
            dispatcher: Arc::new(dispatcher),
        }
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        log::info!("Stopping server...");

        self.running.store(false, Ordering::SeqCst);
        self.net.stop().await
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
                                    .broadcast_event(
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

    pub async fn run(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            panic!("Server is already running")
        }

        self.running.store(true, Ordering::SeqCst);

        log::info!("Starting internal networker...");
        self.net.run().await.unwrap();

        log::info!("Starting client request handler...");
        self.start_client_request_handler();
    }
}
