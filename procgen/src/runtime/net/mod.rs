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

use super::{
    dispatcher::Dispatcher,
    events::{self, Context},
};
use common::packets::*;
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
    guard: MutexGuard<'a, Receiver<Receive>>,
}

impl<'a> Iterator for ConnectionIncoming<'a> {
    type Item = Receive;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.try_recv().ok()
    }
}

#[derive(Debug)]
pub enum Receive {
    Packet(PacketBuffer),
    Disconnect,
}

#[derive(te::Error, Debug)]
#[error("This connection is disconnected and is not running")]
pub struct Disconnected;

struct ConnectionState {
    read: Mutex<BufReader<OwnedReadHalf>>,

    write: Mutex<BufWriter<OwnedWriteHalf>>,
    write_tx: Mutex<Sender<PacketBuffer>>,

    running: AtomicBool,

    compressor: Compressor,
    id: ConnectionId,
}

impl ConnectionState {
    fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub struct Connection {
    state: Arc<ConnectionState>,
    dispatcher: Arc<Dispatcher<Context>>,
}

impl Connection {
    fn start_reader(conn: Self, ctx: Context) {
        tokio::spawn(async move {
            while conn.running() {
                let mut guard = conn.state.read.lock().await;
                let buffer = match conn.state.compressor.read(guard.deref_mut()).await {
                    Ok(raw) => raw,
                    Err(error) => {
                        log::warn!("Error reading packet from {}: {error}", conn.state.id);
                        if let Err(error) = conn.terminate().await {
                            log::warn!("An error occurred while attempting to terminate connection {} due to a different previous error: {error}", conn.id())
                        }
                        return;
                    }
                };

                log::debug!(
                    "Received packet with ID {} from connection {}",
                    buffer.id(),
                    conn.id()
                );

                let ev_conn = conn.clone();

                // TODO: turn this into a macro to keep it DRY
                let result = match buffer.id() {
                    GenerateRegion::ID => match GenerateRegion::from_bincode(&buffer) {
                        Ok(packet) => {
                            let sent = conn
                                .dispatcher
                                .broadcast_event(
                                    ctx.clone(),
                                    events::ReceivedPacket {
                                        connection: ev_conn,
                                        packet: Arc::new(packet),
                                    },
                                )
                                .await;
                            Ok(sent)
                        }
                        Err(error) => Err(error),
                    },

                    GenerateBrush::ID => match GenerateBrush::from_bincode(&buffer) {
                        Ok(packet) => {
                            let sent = conn
                                .dispatcher
                                .broadcast_event(
                                    ctx.clone(),
                                    events::ReceivedPacket {
                                        connection: ev_conn,
                                        packet: Arc::new(packet),
                                    },
                                )
                                .await;
                            Ok(sent)
                        }
                        Err(error) => Err(error),
                    },

                    ListGenerators::ID => match ListGenerators::from_bincode(&buffer) {
                        Ok(packet) => {
                            let sent = conn
                                .dispatcher
                                .broadcast_event(
                                    ctx.clone(),
                                    events::ReceivedPacket {
                                        connection: ev_conn,
                                        packet: Arc::new(packet),
                                    },
                                )
                                .await;
                            Ok(sent)
                        }
                        Err(error) => Err(error),
                    },

                    ProtocolError::ID => match ProtocolError::from_bincode(&buffer) {
                        Ok(packet) => {
                            let sent = conn
                                .dispatcher
                                .broadcast_event(
                                    ctx.clone(),
                                    events::ReceivedPacket {
                                        connection: ev_conn,
                                        packet: Arc::new(packet),
                                    },
                                )
                                .await;
                            Ok(sent)
                        }
                        Err(error) => Err(error),
                    },

                    _ => {
                        log::error!(
                            "Invalid packet ID from connection {}: {}",
                            conn.state.id,
                            buffer.id()
                        );
                        Ok(false)
                    }
                };

                match result {
                    Ok(sent) => {
                        if !sent {
                            log::warn!("Packet from connection {} was decoded, but the event dispatcher did not have a listener to handle it.", conn.id())
                        }
                    }
                    Err(error) => log::error!(
                        "Error when decoding packet from connection {}: {error}",
                        conn.id()
                    ),
                }
            }
        });
    }

    fn start_writer(conn: Self, mut write_rx: Receiver<PacketBuffer>) {
        tokio::spawn(async move {
            while conn.running() {
                if let Ok(raw) = write_rx.try_recv() {
                    if conn
                        .state
                        .compressor
                        .write(&raw, conn.state.write.lock().await.deref_mut())
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        });
    }

    pub fn start(
        stream: TcpStream,
        compression: Compression,
        dispatcher: Arc<Dispatcher<Context>>,
        context: Context,
    ) -> Self {
        let addr = stream.peer_addr().unwrap();
        let (read, write) = stream.into_split();

        let id = {
            match addr {
                SocketAddr::V4(addr) => ConnectionId(addr),
                _ => panic!("invalid address"),
            }
        };

        let (write_tx, write_rx) = tokio::sync::mpsc::channel::<PacketBuffer>(128);

        let state = Arc::new(ConnectionState {
            read: Mutex::new(BufReader::new(read)),

            write: Mutex::new(BufWriter::new(write)),
            write_tx: Mutex::new(write_tx),

            compressor: Compressor::new(compression),
            id,

            running: true.into(),
        });

        let conn = Self { state, dispatcher };

        log::info!("Starting READER for connection {}", conn.state.id);
        Self::start_reader(conn.clone(), context);

        log::info!("Starting WRITER for connection {}", conn.state.id);
        Self::start_writer(conn.clone(), write_rx);

        conn
    }

    pub fn id(&self) -> ConnectionId {
        self.state.id
    }

    pub async fn send_packet<P: Packet>(&self, packet: &P) -> anyhow::Result<()> {
        if !self.running() {
            return Err(Disconnected.into());
        }

        let raw = packet.to_bincode()?;

        self.state.write_tx.lock().await.send(raw).await?;

        Ok(())
    }

    pub fn running(&self) -> bool {
        self.state.running.load(Ordering::SeqCst)
    }

    pub async fn terminate(&self) -> anyhow::Result<()> {
        log::info!("Terminating connection {}", self.id());

        let packet = ProtocolError::fatal(ProtocolErrorKind::Terminated {
            details: "Server stopped".to_string(),
        });

        self.state
            .compressor
            .write(
                &packet.to_bincode().unwrap(),
                self.state.write.lock().await.deref_mut(),
            )
            .await?;

        self.state.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub async fn gentle_error(&self, error: ProtocolErrorKind) -> anyhow::Result<()> {
        self.send_packet(&common::packets::ProtocolError::gentle(error))
            .await
    }

    pub async fn fatal_error(&self, error: ProtocolErrorKind) -> anyhow::Result<()> {
        self.send_packet(&common::packets::ProtocolError::fatal(error))
            .await
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("address", &self.state.id)
            .field("compression", &self.state.compressor.level)
            .field("running", &self.state.running.load(Ordering::SeqCst))
            .finish()
    }
}

#[derive(te::Error, Debug)]
#[error("Listener is not running")]
pub struct ListenerNotRunning;

pub struct Listener {
    inner: Mutex<Option<TcpListener>>,
    dispatcher: Arc<Dispatcher<Context>>,
    running: AtomicBool,
}

impl Listener {
    pub fn new(dispatcher: Arc<Dispatcher<Context>>) -> Self {
        Self {
            inner: Mutex::new(None),
            dispatcher,
            running: false.into(),
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub async fn start(&self, address: SocketAddrV4) -> anyhow::Result<()> {
        *self.inner.lock().await = Some(TcpListener::bind(address).await?);

        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn accept(&self) -> anyhow::Result<(TcpStream, SocketAddr)> {
        if !self.running() {
            Err(ListenerNotRunning.into())
        } else {
            let guard = self.inner.lock().await;

            match guard.as_ref() {
                Some(inner) => Ok(inner.accept().await?),
                None => Err(ListenerNotRunning.into()),
            }
        }
    }
}

pub struct ConnectionRegistry {
    inner: RwLock<HashMap<ConnectionId, Connection>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub async fn get_connection(&self, id: &ConnectionId) -> Option<Connection> {
        self.inner.read().await.get(id).cloned()
    }

    pub async fn add_connection(&self, connection: Connection) {
        self.inner.write().await.insert(connection.id(), connection);
    }

    pub async fn disconnect_all(&self) -> anyhow::Result<()> {
        for conn in self.inner.read().await.values() {
            conn.terminate().await?;
        }

        Ok(())
    }
}
