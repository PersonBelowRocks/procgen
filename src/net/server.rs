use flate2::bufread::ZlibDecoder;
use std::{
    io::Read,
    net::{SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::generate::Generator;

use super::{
    generator_manager::GeneratorManager,
    protocol::{
        DownstreamPacket, GeneratorId, Packet, ProtocolVersion, RequestGenerateChunk,
        RequestRegisterGenerator,
    },
};

pub struct Server {
    reactor: Option<()>,
    listener: Option<Arc<Listener>>,
    connections: Arc<RwLock<Vec<Connection>>>,
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

        let reactor = PacketReactor::new(self.generator_manager.clone());
        let listener = self.listener.unwrap();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connection) = listener.accept_incoming().await {
                    connections.write().await.push(connection);
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
struct Connection {
    stream: BufReader<TcpStream>,
    address: SocketAddr,
}

impl Connection {
    fn new(address: SocketAddr, stream: TcpStream) -> Self {
        Self {
            stream: BufReader::new(stream),
            address,
        }
    }

    fn address(&self) -> SocketAddr {
        self.address
    }

    async fn read_packet<'a>(&mut self) -> DownstreamPacket {
        let length = self.stream.read_u32().await.unwrap();

        let mut compressed_buffer = vec![0u8; length as usize];
        self.stream
            .read_exact(&mut compressed_buffer[..])
            .await
            .unwrap();

        let mut decompressed_buffer = Vec::with_capacity(length as usize);
        ZlibDecoder::new(compressed_buffer.as_slice())
            .read_to_end(&mut decompressed_buffer)
            .unwrap();

        let id = u32::from_be_bytes(decompressed_buffer[0..4].try_into().unwrap());
        let packet_buffer = decompressed_buffer[4..].to_vec();

        match id {
            <RequestRegisterGenerator as Packet>::PACKET_ID => {
                DownstreamPacket::RequestRegisterGenerator(RequestRegisterGenerator::from_bytes(
                    packet_buffer,
                    Default::default(),
                ))
            }
            <RequestGenerateChunk as Packet>::PACKET_ID => DownstreamPacket::RequestGenerateChunk(
                RequestGenerateChunk::from_bytes(packet_buffer, Default::default()),
            ),
            _ => panic!("unrecognized packet id {id}"),
        }
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

    async fn accept_incoming(&self) -> Option<Connection> {
        let (stream, addr) = self.inner.accept().await.ok()?;

        Some(Connection::new(addr, stream))
    }
}
