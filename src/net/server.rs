use flate2::read::ZlibDecoder;
use std::{
    io::Read,
    mem::size_of,
    net::{SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufRead, AsyncReadExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::generate::Generator;

use super::{
    generator_manager::GeneratorManager,
    protocol::{DownstreamPacket, GeneratorId, ProtocolVersion},
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

        let _reactor = PacketReactor::new(self.generator_manager.clone());
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
pub struct AnonymousPacket {
    pub id: u32,
    pub bytes: Box<[u8]>,
}

pub struct PacketCompressor {
    compression_threshold: usize,
}

#[derive(thiserror::Error, Debug)]
enum PacketReadingError {
    #[error("packet claimed to be {0} bytes long uncompressed, but was actually {1}")]
    BadDecompressedLength(usize, usize),
    #[error("attempted to read uncompressed packet with length {0} (below compression threshold), but the claimed decompressed length ({1}) was not equal")]
    MismatchedPacketLengths(usize, usize),
}

impl PacketCompressor {
    pub(self) fn new(compression_threshold: usize) -> Self {
        Self {
            compression_threshold,
        }
    }

    /// Reads a packet's header from a stream, returning the packet's compressed length and decompressed length.
    pub(self) async fn read_header<S>(stream: &mut S) -> anyhow::Result<(usize, usize)>
    where
        S: AsyncBufRead + AsyncReadExt + Unpin,
    {
        let compressed_length = stream.read_u32().await? as usize;
        let decompressed_length = stream.read_u32().await? as usize;

        Ok((compressed_length, decompressed_length))
    }

    /// Read a compressed packet from the stream.
    pub(self) async fn read_compressed<S>(
        stream: &mut S,
        compr_len: usize,
        decompr_len: usize,
    ) -> anyhow::Result<Box<[u8]>>
    where
        S: AsyncBufRead + AsyncReadExt + Unpin,
    {
        let mut compr_buffer = vec![0u8; compr_len];
        let mut decompr_buffer = vec![0u8; decompr_len];

        stream.read_exact(&mut compr_buffer).await?;
        let mut decoder = ZlibDecoder::new(compr_buffer.as_slice());

        decoder.read_exact(&mut decompr_buffer)?;
        if decompr_buffer.len() != decompr_len {
            return Err(PacketReadingError::BadDecompressedLength(
                decompr_len,
                decompr_buffer.len(),
            )
            .into());
        }

        Ok(decompr_buffer.into_boxed_slice())
    }

    /// Read an uncompressed packet from the stream.
    pub(self) async fn read_uncompressed<S>(stream: &mut S, len: usize) -> anyhow::Result<Box<[u8]>>
    where
        S: AsyncBufRead + AsyncReadExt + Unpin,
    {
        let mut buffer = vec![0u8; len];

        stream.read_exact(&mut buffer).await?;

        Ok(buffer.into_boxed_slice())
    }

    /// Read a packet from the provided stream, using the reader's state as context (compression options, etc.)
    /// Returns an [`AnonymousPacket`], which is a struct consisting of a packet's ID and the raw binary data read & decompressed from the stream.
    /// There is no guarantee that the [`AnonymousPacket`] contains valid binary data that corresponds to some packet type, the caller is responsible for
    /// handling potential deserialization of this data (and associated complications/errors).
    pub async fn read_packet<S>(&self, stream: &mut S) -> anyhow::Result<AnonymousPacket>
    where
        S: AsyncBufRead + AsyncReadExt + Unpin,
    {
        // Compressed length, decompressed length
        let (compr_len, decompr_len) = Self::read_header(stream).await?;

        let packet_buffer: Box<[u8]> = if decompr_len > self.compression_threshold {
            Self::read_compressed(stream, compr_len, decompr_len).await?
        } else {
            // If the packet is uncompressed the compressed length and decompressed length should be the same, if they aren't then
            // the client messed up and we're gonna complain about it to them later.
            if compr_len != decompr_len {
                return Err(
                    PacketReadingError::MismatchedPacketLengths(compr_len, decompr_len).into(),
                );
            }
            Self::read_uncompressed(stream, compr_len).await?
        };

        const ID_PREFIX_SIZE: usize = size_of::<u32>();
        let id = u32::from_be_bytes(packet_buffer[..ID_PREFIX_SIZE].try_into().unwrap());

        Ok(AnonymousPacket {
            id,
            bytes: packet_buffer[ID_PREFIX_SIZE..].to_vec().into_boxed_slice(),
        })
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

    async fn read_packet(&mut self) -> DownstreamPacket {
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

    async fn accept_incoming(&self) -> Option<Connection> {
        let (stream, addr) = self.inner.accept().await.ok()?;

        Some(Connection::new(addr, stream))
    }
}

#[cfg(test)]
mod tests {
    use crate::{block::BlockId, chunk::Chunk};

    use super::*;

    pub(self) fn example_chunk() -> Chunk {
        Chunk::try_new(na::vector![2, 2], 320, -64, BlockId::from(0)).unwrap()
    }

    mod packet_compressor {
        use std::io::Write;

        use super::*;
        use flate2::{write::ZlibEncoder, Compression};
        use tokio::io::BufReader;

        #[tokio::test]
        async fn read_header() {
            let mut buffer: Vec<u8> = vec![];

            const COMPRESSED_LENGTH: u32 = 42;
            const DECOMPRESSED_LENGTH: u32 = 21;

            buffer.extend_from_slice(&COMPRESSED_LENGTH.to_be_bytes());
            buffer.extend_from_slice(&DECOMPRESSED_LENGTH.to_be_bytes());

            let mut reader = BufReader::new(buffer.as_slice());
            let (compr_len, decompr_len) =
                PacketCompressor::read_header(&mut reader).await.unwrap();

            assert_eq!(COMPRESSED_LENGTH as usize, compr_len);
            assert_eq!(DECOMPRESSED_LENGTH as usize, decompr_len);
        }

        #[tokio::test]
        async fn read_packet() {
            let mut buffer: Vec<u8> = vec![];

            const PAYLOAD: [u8; 9] = [10, 20, 30, 40, 50, 60, 70, 81, 89];
            const ID: u32 = 20;

            let mut packet_buffer: Vec<u8> = vec![];

            packet_buffer.extend_from_slice(&ID.to_be_bytes());
            packet_buffer.extend_from_slice(PAYLOAD.as_slice());

            let length = packet_buffer.len() as u32;

            buffer.extend_from_slice(&length.to_be_bytes());
            buffer.extend_from_slice(&length.to_be_bytes());
            buffer.extend_from_slice(packet_buffer.as_slice());

            let mut reader = BufReader::new(buffer.as_slice());
            let anon_packet = PacketCompressor::new(42)
                .read_packet(&mut reader)
                .await
                .unwrap();

            assert_eq!(ID, anon_packet.id);
            assert_eq!(PAYLOAD.as_slice(), anon_packet.bytes.as_ref())
        }

        #[tokio::test]
        async fn read_packet_compressed() {
            let mut chunk = example_chunk();
            chunk.set(na::vector![12, 120, 9i32], 60u32.into()).unwrap();
            chunk.set(na::vector![5, 50, 11i32], 42u32.into()).unwrap();

            let payload = bincode::serialize(&chunk).unwrap();
            const ID: u32 = 12;

            let mut packet_buffer: Vec<u8> = vec![];

            packet_buffer.extend_from_slice(&ID.to_be_bytes());
            packet_buffer.extend_from_slice(payload.as_slice());

            let decompressed_length = packet_buffer.len() as u32;

            let mut compressed_packet_buffer: Vec<u8> = vec![];
            let mut compressor =
                ZlibEncoder::new(&mut compressed_packet_buffer, Compression::best());
            compressor.write_all(packet_buffer.as_slice()).unwrap();
            compressor.finish().unwrap();

            let mut buffer: Vec<u8> = vec![];

            buffer.extend_from_slice(
                (compressed_packet_buffer.len() as u32)
                    .to_be_bytes()
                    .as_slice(),
            );
            buffer.extend_from_slice(decompressed_length.to_be_bytes().as_slice());
            buffer.extend_from_slice(compressed_packet_buffer.as_slice());

            let mut reader = BufReader::new(buffer.as_slice());
            let anon_packet = PacketCompressor::new(128)
                .read_packet(&mut reader)
                .await
                .unwrap();

            assert_eq!(ID, anon_packet.id);

            let deserialized_chunk =
                bincode::deserialize::<Chunk>(anon_packet.bytes.as_ref()).unwrap();

            assert_eq!(chunk, deserialized_chunk);
        }
    }
}
