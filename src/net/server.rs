use std::{collections::HashMap, net::SocketAddrV4, sync::Arc};
use threadpool::ThreadPool;
use tokio::{net::TcpListener, sync::oneshot::Receiver};

use crate::{chunk::Chunk, generate::Generator};

use super::{
    generator_manager::GeneratorManager,
    protocol::{GeneratorId, ProtocolVersion, RequestId},
};

pub struct Server {
    reactor: Option<()>,
    listener: Option<TcpListener>,
    version: ProtocolVersion,
    compression_threshold: Option<usize>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            reactor: None,
            listener: None,
            version: Default::default(),
            compression_threshold: None,
        }
    }

    pub fn with_version(mut self, version: ProtocolVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = Some(threshold);
        self
    }

    pub async fn bind(&mut self, address: SocketAddrV4) -> anyhow::Result<()> {
        let listener = TcpListener::bind(address).await?;
        self.listener = Some(listener);

        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        todo!()
    }
}

struct PacketReactor {
    generator_manager: Arc<GeneratorManager>,
}

impl PacketReactor {
    pub fn new(generator_manager: Arc<GeneratorManager>) -> Self {
        Self { generator_manager }
    }
}
