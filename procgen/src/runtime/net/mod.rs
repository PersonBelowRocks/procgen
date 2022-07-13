// pub(crate) mod internal;
pub mod packets;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, SocketAddrV4},
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::{
        mpsc::{Receiver, Sender},
        Mutex, MutexGuard, RwLock,
    },
};

use self::packets::{DowncastPacket, Packet, PacketBuffer, ProtocolError, ProtocolErrorKind};

use super::server::ServerParams;
use common::ConnectionId;

type DynPacket = Box<dyn DowncastPacket>;

type Shared<T> = Arc<RwLock<T>>;
type ConnectionMap = HashMap<ConnectionId, Connection>;

/// Represents a packet header, containing the packet's compressed length and decompressed length.
/// The packet's compressed length is the actual size the packet takes up in the TCP stream.
/// For example, if a header with a compressed length of 20 is sent, that means the next 20 bytes after
/// the header are part of a compressed packet. So a reader should read 20 bytes after the header.
///
/// The decompressed length should be used for error checking and optimizations.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Header {
    pub(crate) compressed_len: u32,
    pub(crate) decompressed_len: u32,
}

impl Header {
    pub(crate) fn new(compressed_len: u32, decompressed_len: u32) -> Self {
        Self {
            compressed_len,
            decompressed_len,
        }
    }

    pub(crate) async fn read<S: AsyncReadExt + Unpin>(s: &mut S) -> anyhow::Result<Self> {
        let compressed_len = s.read_u32().await?;
        let decompressed_len = s.read_u32().await?;

        Ok(Self::new(compressed_len, decompressed_len))
    }

    pub(crate) async fn write<S: AsyncWriteExt + Unpin>(&self, s: &mut S) -> anyhow::Result<()> {
        s.write_u32(self.compressed_len).await?;
        s.write_u32(self.decompressed_len).await?;

        Ok(())
    }

    pub(crate) fn sync_write<S: Write>(&self, s: &mut S) -> anyhow::Result<()> {
        s.write_all(&self.compressed_len.to_be_bytes())?;
        s.write_all(&self.decompressed_len.to_be_bytes())?;

        Ok(())
    }

    pub(crate) fn sync_read<R: Read>(r: &mut R) -> anyhow::Result<Self> {
        let comp_l = {
            let mut buf = [0u8; 4];
            r.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let decomp_l = {
            let mut buf = [0u8; 4];
            r.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        Ok(Self {
            compressed_len: comp_l,
            decompressed_len: decomp_l,
        })
    }
}

#[derive(Copy, Clone)]
pub struct Compressor {
    level: Compression,
}

impl Compressor {
    pub fn new(level: Compression) -> Self {
        Self { level }
    }

    pub async fn write<S: AsyncWriteExt + Unpin>(
        &self,
        packet: &PacketBuffer,
        stream: &mut S,
    ) -> anyhow::Result<()> {
        let decompressed_len = packet.len() as u32;

        let compressed_buf = {
            let mut buf = Vec::<u8>::new();
            let mut encoder = ZlibEncoder::new(&mut buf, self.level);
            encoder.write_all(packet.as_ref())?;
            encoder.finish()?;

            buf
        };

        let compressed_len = compressed_buf.len() as u32;

        Header::new(compressed_len, decompressed_len)
            .write(stream)
            .await?;
        stream.write_all(&compressed_buf).await?;
        stream.flush().await?;

        Ok(())
    }

    pub async fn read<S: AsyncReadExt + Unpin>(
        &self,
        stream: &mut S,
    ) -> anyhow::Result<PacketBuffer> {
        let header = Header::read(stream).await?;

        let compressed_buf = {
            let mut buf = vec![0u8; header.compressed_len as usize];

            stream.read_exact(&mut buf).await?;

            buf
        };

        let mut decompressor = ZlibDecoder::new(&compressed_buf[..]);

        let buf = PacketBuffer::from_reader(&mut decompressor)?;
        Ok(buf)
    }
}

pub struct ConnectionIncoming<'a> {
    guard: MutexGuard<'a, Receiver<PacketBuffer>>,
}

impl<'a> Iterator for ConnectionIncoming<'a> {
    type Item = PacketBuffer;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.try_recv().ok()
    }
}

#[derive(Clone)]
pub struct Connection {
    read: Arc<Mutex<BufReader<OwnedReadHalf>>>,
    read_rx: Option<Arc<Mutex<Receiver<PacketBuffer>>>>,

    write: Arc<Mutex<BufWriter<OwnedWriteHalf>>>,
    write_tx: Option<Arc<Mutex<Sender<PacketBuffer>>>>,

    running: Arc<AtomicBool>,

    compressor: Compressor,
    id: ConnectionId,
}

impl Connection {
    pub fn new(stream: TcpStream, compression: Compression) -> Self {
        let addr = stream.peer_addr().unwrap();
        let (read, write) = stream.into_split();

        let id = {
            match addr {
                SocketAddr::V4(addr) => ConnectionId(addr),
                _ => panic!("invalid address"),
            }
        };

        Self {
            read: Mutex::new(BufReader::new(read)).into(),
            read_rx: None,
            write: Mutex::new(BufWriter::new(write)).into(),
            write_tx: None,
            running: Arc::new(false.into()),
            compressor: Compressor::new(compression),
            id,
        }
    }

    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub async fn send_packet<P: Packet>(&self, packet: &P) -> anyhow::Result<()> {
        let raw = packet.to_bincode()?;

        self.write_tx
            .as_ref()
            .unwrap()
            .lock()
            .await
            .send(raw)
            .await?;

        Ok(())
    }

    pub async fn incoming(&self) -> ConnectionIncoming<'_> {
        ConnectionIncoming {
            guard: self.read_rx.as_ref().unwrap().lock().await,
        }
    }

    pub fn run(&mut self) {
        assert!(
            !self.running.load(Ordering::SeqCst),
            "cannot run connection twice!"
        );

        self.running.store(true, Ordering::SeqCst);

        let (read_tx, read_rx) = tokio::sync::mpsc::channel::<PacketBuffer>(128);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<PacketBuffer>(128);

        self.read_rx = Some(Arc::new(Mutex::new(read_rx)));
        self.write_tx = Some(Arc::new(Mutex::new(write_tx)));

        // Reader
        let reader = self.read.clone();
        let compressor = self.compressor;
        let running = self.running.clone();
        let id = self.id();
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                for _ in 0..100 {
                    let mut guard = reader.lock().await;
                    match compressor.read(guard.deref_mut()).await {
                        Ok(raw) => read_tx.send(raw).await.unwrap(),
                        Err(error) => {
                            log::warn!("error reading packet from {id}: {error}")
                        }
                    }
                }
            }
        });

        // Writer
        let writer = self.write.clone();
        let compressor = self.compressor;
        let running = self.running.clone();
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                for _ in 0..100 {
                    if let Ok(raw) = write_rx.try_recv() {
                        compressor
                            .write(&raw, writer.lock().await.deref_mut())
                            .await
                            .unwrap();
                    }
                }
            }
        });
    }

    pub async fn terminate(&self) -> anyhow::Result<()> {
        let packet = ProtocolError::fatal(ProtocolErrorKind::Terminated {
            details: "Server stopped".to_string(),
        });

        self.compressor
            .write(
                &packet.to_bincode().unwrap(),
                self.write.lock().await.deref_mut(),
            )
            .await?;

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("address", &self.id)
            .field("compression", &self.compressor.level)
            .field("running", &self.running.load(Ordering::SeqCst))
            .finish()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Params {
    pub(crate) addr: SocketAddrV4,
    pub(crate) compression: Compression,
}

impl From<ServerParams> for Params {
    fn from(p: ServerParams) -> Self {
        Self {
            addr: p.addr,
            compression: p.compression,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Networker {
    params: Params,
    listener: Option<Arc<Mutex<TcpListener>>>,
    connections: Shared<ConnectionMap>,
    running: Arc<AtomicBool>,
}

impl Networker {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            listener: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(false.into()),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        assert!(
            !self.running.load(Ordering::SeqCst),
            "cannot start networker twice!"
        );

        self.running.store(true, Ordering::SeqCst);

        let listener = Arc::new(Mutex::new(TcpListener::bind(self.params.addr).await?));
        self.listener = Some(listener.clone());

        let connections = self.connections.clone();
        let compression = self.params.compression;
        let running = self.running.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                for _ in 0..100 {
                    let (incoming, _) = listener.lock().await.accept().await.unwrap();

                    let mut conn = Connection::new(incoming, compression);

                    log::info!("accepted connection from {}", conn.id());

                    conn.run();
                    connections.write().await.insert(conn.id(), conn);
                }
            }
        });

        Ok(())
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);

        for conn in self.connections.read().await.values() {
            if let Err(error) = conn.terminate().await {
                log::warn!("Error when terminating connection {conn:?}: {error}");
            }
        }

        Ok(())
    }

    pub async fn incoming(&self) -> Incoming {
        let guard = self.connections.read().await;
        let mut packets = Vec::new();

        for (_, conn) in guard.iter() {
            packets.extend(
                conn.incoming()
                    .await
                    .map(|p| (conn.clone(), packets::parse_dyn(&p))),
            );
        }

        Incoming(packets.into_iter())
    }

    #[inline]
    pub async fn connection(&self, id: ConnectionId) -> Option<Connection> {
        self.connections.read().await.get(&id).cloned()
    }
}

pub struct Incoming(std::vec::IntoIter<(Connection, anyhow::Result<DynPacket>)>);

impl Iterator for Incoming {
    type Item = (Connection, anyhow::Result<DynPacket>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
