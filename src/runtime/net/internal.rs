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

type ConnectionHandle = (tokio::sync::mpsc::Sender<AddressedPacket>, Arc<Connection>);

pub(super) async fn run(params: Params, internal: InternalNetworkerHandle) -> ! {
    // TODO: this is essentially #[tokio::main] but we manually build the runtime and submit this as the "main" function to it.
    // this function should set up all the networking stuff and then diverge into just serving terrain data over TCP.

    let server = Server::create(params.addr).await;
    let connections: Shared<HashMap<u32, ConnectionHandle>> = Arc::new(RwLock::new(HashMap::new()));

    tokio::spawn(server.run(params, connections.clone()));

    loop {
        todo!()
    }
}

struct ServerHandle {
    outbound_tx: tokio::sync::mpsc::Sender<AddressedPacket>,
    inbound_rx: tokio::sync::mpsc::Receiver<AddressedPacket>,
}

struct Server {
    listener: TcpListener,
    connections: Shared<HashMap<u32, ConnectionHandle>>,
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
    async fn accept(&self, id: u32) -> anyhow::Result<Connection> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Connection::new(id, stream))
    }

    async fn run(
        self,
        params: Params,
        connections: Shared<HashMap<u32, ConnectionHandle>>,
    ) -> ServerHandle {
        let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);

        tokio::spawn(async move {
            loop {
                // random unique ID for the next connection
                let random_id = {
                    let mut id = 0;
                    let guard = connections.read().await;

                    while !guard.contains_key(&id) {
                        id = rand::random::<u32>();
                    }
                    id
                };

                tokio::select! {
                    incoming = self.accept(random_id) => {
                        let shared_connection = Arc::new(incoming.unwrap());

                        let (l_outbound_tx, l_outbound_rx) = tokio::sync::mpsc::channel::<AddressedPacket>(128);

                        self.connections.write().await.insert(
                            random_id,
                            (l_outbound_tx, shared_connection.clone()),
                        );

                        let connections_copy = self.connections.clone();
                        let ibtx_copy = inbound_tx.clone();
                        tokio::spawn(async move {
                            let task = tokio::spawn(Connection::run(
                                shared_connection.clone(),
                                params.compression,
                                ibtx_copy,
                                l_outbound_rx,
                            ));

                            log_connection_error(task.await, shared_connection.clone());

                            connections_copy.write().await.remove(&shared_connection.id);
                        });
                    },
                }
            }
        });

        todo!()
    }
}

/// Represents a packet header, containing the packet's compressed length and decompressed length.
/// The packet's compressed length is the actual size the packet takes up in the TCP stream.
/// For example, if a header with a compressed length of 20 is sent, that means the next 20 bytes after
/// the header are part of a compressed packet. So a reader should read 20 bytes after the header.
///
/// The decompressed length should be used for error checking and optimizations.
#[derive(Copy, Clone, Debug)]
struct Header {
    compressed_len: u32,
    decompressed_len: u32,
}

impl Header {
    fn new(compressed_len: u32, decompressed_len: u32) -> Self {
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
}

struct RawPacket {
    id: u16,
    body: Vec<u8>,
}

struct Connection {
    id: u32,
    address: SocketAddr,
    reader: tokio::sync::Mutex<BufReader<OwnedReadHalf>>,
    writer: tokio::sync::Mutex<BufWriter<OwnedWriteHalf>>,
}

impl Connection {
    fn new(id: u32, stream: TcpStream) -> Self {
        let (r, w) = stream.into_split();

        Self {
            id,
            address: r.peer_addr().unwrap(),
            reader: tokio::sync::Mutex::new(BufReader::new(r)),
            writer: tokio::sync::Mutex::new(BufWriter::new(w)),
        }
    }

    async fn run(
        this: Arc<Self>,
        compression: Compression,
        inbound_tx: tokio::sync::mpsc::Sender<AddressedPacket>,
        mut outbound_rx: tokio::sync::mpsc::Receiver<AddressedPacket>,
    ) -> anyhow::Result<()> {
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

                inbound_tx
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
