use std::{any::type_name, collections::HashMap, io::Write, net::SocketAddr};

use anyhow::Error;
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::RwLock,
    task::{JoinError, JoinHandle},
};

use super::{packets::Packet, *};

type Shared<T> = Arc<RwLock<T>>;

fn log_connection_error(
    result: Result<Result<(), anyhow::Error>, JoinError>,
    conn: Arc<Connection>,
) {
    match result {
        Ok(result) => match result {
            Ok(_) => {
                log::info!("Connection loop for connection {:?} ended without errors, terminating connection", conn);
            }
            Err(error) => {
                log::warn!("Connection loop for connection {:?} ended with an error: {}. Terminating connection.", conn, error);
            }
        },
        Err(error) => {
            log::warn!("Encountered JoinError while running connection loop for connection {:?}, terminating connection. Error: {}", conn, error)
        }
    }
}

pub(super) async fn run(params: Params, internal: InternalNetworkerHandle) -> ! {
    // TODO: test!!!

    let server = Server::create(params.addr).await;

    let mut handle = server.run(params);
    // This loop basically just transfers packets from the async stream into the sync stream, and vice versa.
    loop {
        if let Some(packet) = internal.receive() {
            handle.outbound_tx.send(packet).await.unwrap();
        }

        if let Ok(packet) = handle.inbound_rx.try_recv() {
            internal.send(packet);
        }
    }
}

struct ServerHandle {
    outbound_tx: tokio::sync::mpsc::Sender<AddressedPacket>,
    inbound_rx: tokio::sync::mpsc::Receiver<AddressedPacket>,
}

struct Server {
    listener: TcpListener,
    connections: Shared<HashMap<u32, Arc<Connection>>>,
}

impl Server {
    async fn create(addr: SocketAddrV4) -> Self {
        Self {
            listener: TcpListener::bind(addr)
                .await
                .expect("couldn't bind to address"),

            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Accept an incoming connection.
    async fn accept(&self) -> Result<TcpStream, std::io::Error> {
        self.listener.accept().await.map(|a| a.0)
    }

    async fn random_id(&self) -> u32 {
        let mut id = 0;
        let guard = self.connections.read().await;

        while guard.contains_key(&id) {
            id = rand::random::<u32>();
        }
        id
    }

    async fn handle_incoming(
        &mut self,
        incoming: TcpStream,
        params: Params,
        inbound_tx: tokio::sync::mpsc::Sender<AddressedPacket>,
    ) {
        // random unique ID for the next connection
        let random_id = self.random_id().await;

        let conn = Connection::new(random_id, incoming);

        let shared_connection = Arc::new(conn);

        self.connections
            .write()
            .await
            .insert(random_id, shared_connection.clone());

        let connections_copy = self.connections.clone();
        tokio::spawn(async move {
            let task = tokio::spawn(Connection::run(
                shared_connection.clone(),
                params.compression,
                inbound_tx,
            ));

            log_connection_error(task.await, shared_connection.clone());

            connections_copy.write().await.remove(&shared_connection.id);
        });
    }

    fn run(mut self, params: Params) -> ServerHandle {
        let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);

        let _handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    incoming_stream = self.accept() => {
                        self.handle_incoming(incoming_stream.unwrap(), params, inbound_tx.clone()).await;
                    },
                    outbound = outbound_rx.recv() => {
                        if let Some(outbound) = outbound {
                            if let Some(conn) = self.connections.read().await.get(&outbound.caller_id) {
                                conn.send_packet(outbound).await;
                            }
                        }
                    },
                };
            }
        });

        ServerHandle {
            outbound_tx,
            inbound_rx,
        }
    }
}

/// Represents a packet header, containing the packet's compressed length and decompressed length.
/// The packet's compressed length is the actual size the packet takes up in the TCP stream.
/// For example, if a header with a compressed length of 20 is sent, that means the next 20 bytes after
/// the header are part of a compressed packet. So a reader should read 20 bytes after the header.
///
/// The decompressed length should be used for error checking and optimizations.
#[derive(Copy, Clone, Debug)]
pub(super) struct Header {
    compressed_len: u32,
    decompressed_len: u32,
}

impl Header {
    pub(super) fn new(compressed_len: u32, decompressed_len: u32) -> Self {
        Self {
            compressed_len,
            decompressed_len,
        }
    }

    async fn read<S: AsyncRead + AsyncReadExt + Unpin>(s: &mut S) -> anyhow::Result<Self> {
        let compressed_len = s.read_u32().await?;
        let decompressed_len = s.read_u32().await?;

        Ok(Self::new(compressed_len, decompressed_len))
    }

    async fn write<S: AsyncWrite + AsyncWriteExt + Unpin>(&self, s: &mut S) -> anyhow::Result<()> {
        s.write_u32(self.compressed_len).await?;
        s.write_u32(self.decompressed_len).await?;

        Ok(())
    }

    pub(super) fn sync_write<S: Write>(&self, s: &mut S) -> anyhow::Result<()> {
        s.write_all(&self.compressed_len.to_be_bytes())?;
        s.write_all(&self.decompressed_len.to_be_bytes())?;

        Ok(())
    }
}

struct RawPacket {
    id: u16,
    body: Vec<u8>,
}

struct Connection {
    id: u32,
    address: SocketAddr,
    outbound_tx: RwLock<Option<tokio::sync::mpsc::Sender<AddressedPacket>>>,
    inbound_tx: RwLock<Option<tokio::sync::mpsc::Sender<AddressedPacket>>>,
    reader: tokio::sync::Mutex<BufReader<OwnedReadHalf>>,
    writer: tokio::sync::Mutex<BufWriter<OwnedWriteHalf>>,
}

impl Connection {
    fn new(id: u32, stream: TcpStream) -> Self {
        let (r, w) = stream.into_split();

        Self {
            id,
            address: r.peer_addr().unwrap(),
            outbound_tx: RwLock::new(None),
            inbound_tx: RwLock::new(None),
            reader: tokio::sync::Mutex::new(BufReader::new(r)),
            writer: tokio::sync::Mutex::new(BufWriter::new(w)),
        }
    }

    async fn run(
        this: Arc<Self>,
        compression: Compression,
        inbound_tx: tokio::sync::mpsc::Sender<AddressedPacket>,
    ) -> anyhow::Result<()> {
        let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);

        *this.outbound_tx.write().await = Some(outbound_tx);
        *this.inbound_tx.write().await = Some(inbound_tx);

        let this1 = this.clone();
        let read: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            let this = this1;
            loop {
                use super::packets::*;
                let packet = this.read_packet().await?;

                let dyn_packet: DynPacket = match packet.id {
                    GenerateChunk::ID => {
                        let packet: GenerateChunk = bincode::deserialize(&packet.body)?;
                        Box::new(packet)
                    }
                    AddGenerator::ID => {
                        let packet: AddGenerator = bincode::deserialize(&packet.body)?;
                        Box::new(packet)
                    }
                    _ => {
                        anyhow::bail!("Received invalid packet ID")
                    }
                };

                this.inbound_tx
                    .read()
                    .await
                    .as_ref()
                    .unwrap()
                    .send(AddressedPacket::new(this.id, dyn_packet))
                    .await
                    .unwrap();
            }
        });

        let write: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                use super::packets::*;
                if let Some(packet) = outbound_rx.recv().await {
                    if let Some(packet) = packet.packet.downcast_ref::<ReplyChunk>() {
                        this.write_packet(packet, compression).await?;
                    } else if let Some(packet) =
                        packet.packet.downcast_ref::<ConfirmGeneratorAddition>()
                    {
                        this.write_packet(packet, compression).await?;
                    }
                }
            }
        });

        tokio::select! {
            res = read => res?,
            res = write => res?,
        }
    }

    /// Send a packet to the client of this connection.
    /// # Panics
    /// Panics if the connection is not running (i.e., [`Connection::run`] has not been called)
    async fn send_packet(&self, packet: AddressedPacket) {
        self.outbound_tx
            .read()
            .await
            .as_ref()
            .expect("connection is not running")
            .send(packet)
            .await
            .unwrap();
    }

    /// Attempt to read a packet from this connection's internal stream.
    /// If this returns an error the connection should be terminated and the error should be logged.
    async fn read_packet(&self) -> anyhow::Result<RawPacket> {
        let mut reader = self.reader.lock().await;

        let header = Header::read(&mut *reader).await?;

        let mut compressed_buffer = vec![0u8; header.compressed_len as usize];
        reader.read_exact(&mut compressed_buffer).await?;

        let decompressed_buffer = {
            use std::io::Read;

            let mut buf = Vec::<u8>::with_capacity(header.decompressed_len as usize);

            let mut decoder = ZlibDecoder::new(&compressed_buffer[..]);
            if decoder.read_to_end(&mut buf)? as u32 != header.decompressed_len {
                anyhow::bail!(
                    "decompressed length in header did not match amount of bytes decompressed"
                )
            }

            buf
        };

        let id = u16::from_be_bytes((&decompressed_buffer[..2]).try_into()?);
        let bytes = decompressed_buffer[2..].to_vec();

        Ok(RawPacket { id, body: bytes })
    }

    /// Attempt to write a packet to this connection's internal stream with the provided `compression` level.
    /// If this returns an error the connection should be terminated and the error should be logged.
    async fn write_packet<P: Packet>(
        &self,
        packet: &P,
        compression: Compression,
    ) -> anyhow::Result<()> {
        let decompressed_buffer = bincode::serialize(packet)?;
        let decompressed_len = decompressed_buffer.len() as u32;

        let compressed_buffer = {
            let mut buf = Vec::<u8>::new();
            let mut encoder = ZlibEncoder::new(&mut buf, compression);
            encoder.write_all(&decompressed_buffer)?;
            encoder.finish()?;

            buf
        };
        let compressed_len = compressed_buffer.len() as u32;

        let header = Header::new(compressed_len, decompressed_len);

        let mut writer = self.writer.lock().await;

        header.write(&mut *writer).await?;
        writer.write_all(&compressed_buffer).await?;
        writer.flush().await?;

        Ok(())
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("address", &self.address)
            .field("id", &self.id)
            .finish()
    }
}
