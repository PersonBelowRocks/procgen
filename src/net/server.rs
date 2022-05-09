use std::{
    net::{SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::generate::Generator;

use super::{
    compressor::PacketCompressor,
    generator_manager::GeneratorManager,
    protocol::{DownstreamPacket, GeneratorId, ProtocolVersion},
};

pub struct Server {
    reactor: Option<()>,
    listener: Option<Arc<Listener>>,
    connections: Arc<RwLock<Vec<Connection<TcpStream>>>>,
    generator_manager: Arc<RwLock<GeneratorManager>>,
    version: ProtocolVersion,
    compression_threshold: Option<usize>,
    interrupt: bool,
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ServerError {
    #[error("server does not have a set address")]
    AddressNotBound,
}

impl Server {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            reactor: None,
            listener: None,
            version: Default::default(),
            compression_threshold: None,
            generator_manager: Arc::new(RwLock::new(GeneratorManager::new())),
            interrupt: false,
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

    pub fn with_generator<T: Generator + 'static>(self, id: GeneratorId, generator: T) -> Self {
        let generator_manager = self.generator_manager.clone();
        generator_manager
            .blocking_write()
            .add_generator(id, generator)
            .unwrap();
        self
    }

    pub async fn bind(&mut self, address: SocketAddrV4) -> anyhow::Result<()> {
        let listener = Listener::new(TcpListener::bind(address).await?);
        self.listener = Some(Arc::new(listener));

        Ok(())
    }

    pub async fn run(self) -> anyhow::Result<()> {
        if self.listener.is_none() {
            return Err(ServerError::AddressNotBound.into());
        }

        let _reactor = PacketReactor::new(self.generator_manager.clone());
        let listener = self.listener.unwrap();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connection) = listener.accept_incoming().await {
                    connections.write().await.push(connection.clone());
                    tokio::spawn(async move {
                        dbg!(connection.address());
                    });
                }
            }
        });

        loop {
            if self.interrupt {
                break;
            }
        }

        Ok(())
    }
}

struct PacketReactor {
    generator_manager: Arc<RwLock<GeneratorManager>>,
}

impl PacketReactor {
    pub fn new(generator_manager: Arc<RwLock<GeneratorManager>>) -> Self {
        Self { generator_manager }
    }
}

#[derive(Debug)]
pub struct AnonymousPacket {
    pub id: u32,
    pub bytes: Box<[u8]>,
}

/// Convenience trait implemented for all types that are [`AsyncRead`] + [`AsyncReadExt`] + [`AsyncWrite`] + [`AsyncWriteExt`].
/// Doesn't do anything on its own but acts as a convenience trait so you don't need to write the long bounds above.
trait AsyncStream: AsyncRead + AsyncReadExt + AsyncWrite + AsyncWriteExt {}
impl<S> AsyncStream for S where S: AsyncRead + AsyncReadExt + AsyncWrite + AsyncWriteExt {}

// This struct is generic so that we can use mock streams for testing and TCP streams in the actual program.
#[derive(Debug)]
struct Connection<S>
where
    S: AsyncStream,
{
    stream: Arc<RwLock<BufReader<S>>>,
    compressor: PacketCompressor,
    address: SocketAddrV4,
}

// Rust didn't want to derive clone for some reason so we gotta implement it ourselves.
impl<S> Clone for Connection<S>
where
    S: AsyncStream,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            address: self.address,
        }
    }
}

impl<S> Connection<S>
where
    S: AsyncStream,
{
    fn new(address: SocketAddrV4, stream: S) -> Self {
        let stream = BufReader::new(stream);

        Self {
            stream: Arc::new(RwLock::new(stream)),
            address,
        }
    }

    fn address(&self) -> SocketAddrV4 {
        self.address
    }

    fn can_write(&self) -> bool {
        self.stream.try_write().is_ok()
    }

    async fn read_packet(&mut self) -> DownstreamPacket {
        todo!()
    }

    async fn write_packet(&mut self) {
        todo!()
    }
}

#[derive(Debug)]
struct Listener {
    inner: TcpListener,
}

impl Listener {
    fn new(inner: TcpListener) -> Self {
        Self { inner }
    }

    async fn accept_incoming(&self) -> Option<Connection<TcpStream>> {
        let (stream, addr) = self.inner.accept().await.ok()?;

        let addr = match addr {
            SocketAddr::V4(a) => a,
            _ => unreachable!(),
        };

        Some(Connection::new(addr, stream))
    }
}
