// pub(crate) mod internal;
pub mod packets;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, SocketAddrV4},
    ops::DerefMut,
    sync::Arc,
};

use flate2::{
    write::{ZlibDecoder, ZlibEncoder},
    Compression,
};
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

use self::packets::{DowncastPacket, Packet};

use super::{server::ServerParams, util::ConnectionId};

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
        packet: &[u8],
        stream: &mut S,
    ) -> anyhow::Result<()> {
        let decompressed_len = packet.len() as u32;

        let compressed_buf = {
            let mut buf = Vec::<u8>::new();
            let mut encoder = ZlibEncoder::new(&mut buf, self.level);
            encoder.write_all(packet)?;
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

    pub async fn read<S: AsyncReadExt + Unpin>(&self, stream: &mut S) -> anyhow::Result<Vec<u8>> {
        let header = Header::read(stream).await?;

        let compressed_buf = {
            let mut buf = vec![0u8; header.compressed_len as usize];

            stream.read_exact(&mut buf).await?;

            buf
        };

        let decompressed_buf = {
            let mut buf = Vec::with_capacity(header.decompressed_len as usize);

            let mut decoder = ZlibDecoder::new(&mut buf);
            decoder.write_all(&compressed_buf)?;
            decoder.finish()?;

            buf
        };

        Ok(decompressed_buf)
    }
}

pub struct ConnectionIncoming<'a> {
    guard: MutexGuard<'a, Receiver<Vec<u8>>>,
}

impl<'a> Iterator for ConnectionIncoming<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.try_recv().ok()
    }
}

#[derive(Clone)]
pub struct Connection {
    read: Arc<Mutex<BufReader<OwnedReadHalf>>>,
    read_rx: Option<Arc<Mutex<Receiver<Vec<u8>>>>>,

    write: Arc<Mutex<BufWriter<OwnedWriteHalf>>>,
    write_tx: Option<Arc<Mutex<Sender<Vec<u8>>>>>,

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
            compressor: Compressor::new(compression),
            id,
        }
    }

    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub async fn send_packet<P: Packet>(&self, packet: &P) -> anyhow::Result<()> {
        let raw = packet.bincode();

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
            (self.read_rx.is_none() && self.write_tx.is_none()),
            "cannot run connection twice!"
        );

        let (read_tx, read_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128);

        self.read_rx = Some(Arc::new(Mutex::new(read_rx)));
        self.write_tx = Some(Arc::new(Mutex::new(write_tx)));

        // Reader
        let reader = self.read.clone();
        let compressor = self.compressor;
        tokio::spawn(async move {
            loop {
                let raw = compressor
                    .read(reader.lock().await.deref_mut())
                    .await
                    .unwrap();
                read_tx.send(raw).await.unwrap();
            }
        });

        // Writer
        let writer = self.write.clone();
        let compressor = self.compressor;
        tokio::spawn(async move {
            loop {
                if let Some(raw) = write_rx.recv().await {
                    compressor
                        .write(&raw, writer.lock().await.deref_mut())
                        .await
                        .unwrap();
                }
            }
        });
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
}

impl Networker {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            listener: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        assert!(self.listener.is_none(), "cannot start networker twice!");

        let listener = Arc::new(Mutex::new(TcpListener::bind(self.params.addr).await?));
        self.listener = Some(listener.clone());

        let connections = self.connections.clone();
        let compression = self.params.compression;

        tokio::spawn(async move {
            loop {
                let (incoming, _) = listener.lock().await.accept().await.unwrap();

                let mut conn = Connection::new(incoming, compression);

                log::info!("accepted connection from {}", conn.id());

                conn.run();
                connections.write().await.insert(conn.id(), conn);
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        todo!()
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
