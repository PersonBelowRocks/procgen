mod internal;
pub mod packets;

use std::{
    net::SocketAddrV4,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, MutexGuard,
    },
};

use flate2::Compression;

use self::packets::DowncastPacket;

type DynPacket = Box<dyn DowncastPacket>;
type ChannelData = AddressedPacket;

#[derive(Debug)]
pub struct AddressedPacket {
    packet: DynPacket,
    caller_id: u32,
}

impl AddressedPacket {
    fn new(id: u32, packet: DynPacket) -> Self {
        Self {
            packet,
            caller_id: id,
        }
    }
}

#[derive(Clone)]
pub(crate) struct NetworkerHandle {
    inbound: Arc<Mutex<Receiver<ChannelData>>>,
    outbound: Sender<ChannelData>,
}

impl NetworkerHandle {
    fn new(rx_inbound: Receiver<ChannelData>, tx_outbound: Sender<ChannelData>) -> Self {
        Self {
            inbound: Arc::new(Mutex::new(rx_inbound)),
            outbound: tx_outbound,
        }
    }
}

#[derive(Clone)]
pub(self) struct InternalNetworkerHandle {
    inbound: Arc<Mutex<Sender<ChannelData>>>,
    outbound: Arc<Mutex<Receiver<ChannelData>>>,
}

impl InternalNetworkerHandle {
    fn new(tx_inbound: Sender<ChannelData>, rx_outbound: Receiver<ChannelData>) -> Self {
        Self {
            inbound: Arc::new(Mutex::new(tx_inbound)),
            outbound: Arc::new(Mutex::new(rx_outbound)),
        }
    }

    fn send(&self, packet: ChannelData) {
        self.inbound.lock().unwrap().send(packet).unwrap();
    }

    fn receive(&self) -> Option<ChannelData> {
        self.outbound.lock().unwrap().try_recv().ok()
    }
}

fn make_handles() -> (NetworkerHandle, InternalNetworkerHandle) {
    let (tx_i, rx_i) = mpsc::channel::<ChannelData>();
    let (tx_o, rx_o) = mpsc::channel::<ChannelData>();

    (
        NetworkerHandle::new(rx_i, tx_o),
        InternalNetworkerHandle::new(tx_i, rx_o),
    )
}

#[derive(Copy, Clone)]
pub(crate) struct Params {
    pub(crate) addr: SocketAddrV4,
    pub(crate) compression: Compression,
}

pub(crate) struct Networker {
    runtime: Arc<tokio::runtime::Runtime>,
    handle: Option<NetworkerHandle>,
}

impl Networker {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ),
            handle: None,
        }
    }

    pub fn run(&mut self, params: Params) {
        let (external, internal) = make_handles();
        self.handle = Some(external);

        // let guard = self.runtime.enter();
        // Box::leak(Box::new(guard));
        // let handle = self.runtime.spawn(internal::run(params, internal));
        let rt = self.runtime.clone();
        std::thread::spawn(move || {
            let _guard = rt.enter();
            rt.block_on(internal::run(params, internal));
        });

        // self.handle.clone().unwrap()
    }

    pub fn handle(&self) -> NetworkerHandle {
        self.handle.clone().unwrap()
    }

    #[inline]
    pub fn send(&self, packet: AddressedPacket) {
        self.handle
            .as_ref()
            .expect("Networker must be started with Networker::run() before packets are sent")
            .outbound
            .send(packet)
            .unwrap()
    }

    #[inline]
    pub fn poll(&self) -> Option<AddressedPacket> {
        self.handle
            .as_ref()
            .expect("Networker must be started with Networker::run() before packets are read")
            .inbound
            .lock()
            .unwrap()
            .try_recv()
            .ok()
    }

    #[inline]
    pub fn incoming(&self) -> Incoming<'_> {
        Incoming {
            guard: self
                .handle
                .as_ref()
                .expect("Networker must be started with Networker::run() before packets are read")
                .inbound
                .lock()
                .unwrap(),
        }
    }
}

pub struct Incoming<'a> {
    guard: MutexGuard<'a, Receiver<AddressedPacket>>,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = AddressedPacket;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.guard.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
    };

    use flate2::{read::ZlibDecoder, write::ZlibEncoder};
    use volume::Volume;

    use crate::chunk::{Chunk, Spaces};

    use super::{
        internal::Header,
        packets::{GenerateChunk, Packet, ReplyChunk},
        *,
    };

    // TODO: this test is great and all, but we should also have some tests for more abnormal behaviour, like malformed packets
    #[test]
    fn networker_recv() {
        let mut networker = Networker::new();

        let params = Params {
            addr: "0.0.0.0:33445".parse().unwrap(),
            compression: Compression::best(),
        };

        networker.run(params);

        let mut stream =
            TcpStream::connect("127.0.0.1:33445".parse::<SocketAddrV4>().unwrap()).unwrap();
        let packet = packets::AddGenerator {
            request_id: 42,
            name: "hello!!!".to_string(),
        };

        let packet_id = packets::AddGenerator::ID;
        let packet_body = bincode::serialize(&packet).unwrap();

        let mut uncompressed_buf = packet_id.to_be_bytes().to_vec();
        uncompressed_buf.extend(packet_body);

        let decompressed_len = uncompressed_buf.len();

        let mut compressed_buf = Vec::<u8>::new();
        let mut compressor = ZlibEncoder::new(&mut compressed_buf, Compression::best());
        compressor.write_all(&uncompressed_buf).unwrap();
        compressor.finish().unwrap();

        let compressed_len = compressed_buf.len();

        let header = Header::new(compressed_len as u32, decompressed_len as u32);

        header.sync_write(&mut stream).unwrap();
        stream.write_all(&compressed_buf).unwrap();

        match networker
            .handle()
            .inbound
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(1))
        {
            Ok(p) => {
                let packet = p.packet.downcast_ref::<packets::AddGenerator>().unwrap();
                assert_eq!(packet.name, "hello!!!");
                assert_eq!(packet.request_id, 42);
            }

            Err(error) => {
                panic!("Receive error in networker handle: {}", error);
            }
        };
    }

    #[test]
    fn networker_send() {
        let mut networker = Networker::new();

        let params = Params {
            addr: "0.0.0.0:33446".parse().unwrap(),
            compression: Compression::best(),
        };

        networker.run(params);

        let mut stream =
            TcpStream::connect("127.0.0.1:33446".parse::<SocketAddrV4>().unwrap()).unwrap();

        let generate_chunk_packet = GenerateChunk {
            request_id: 560,
            generator_id: 4,
            pos: na::vector![-6, 2],
        };

        let packet_id = GenerateChunk::ID;
        let packet_body = bincode::serialize(&generate_chunk_packet).unwrap();

        {
            let mut uncompressed_buf = packet_id.to_be_bytes().to_vec();
            uncompressed_buf.extend(packet_body);

            let decompressed_len = uncompressed_buf.len();

            let mut compressed_buf = Vec::<u8>::new();
            let mut compressor = ZlibEncoder::new(&mut compressed_buf, Compression::best());
            compressor.write_all(&uncompressed_buf).unwrap();
            compressor.finish().unwrap();

            let compressed_len = compressed_buf.len();

            let header = Header::new(compressed_len as u32, decompressed_len as u32);

            header.sync_write(&mut stream).unwrap();
            stream.write_all(&compressed_buf).unwrap();
        }

        let client_id = networker
            .handle()
            .inbound
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .caller_id;

        let mut chunk = Chunk::new(77.into(), na::vector![-6, 2], -64, 320);
        chunk.set(Spaces::Cs([10i32, 120, 8]), 80.into());
        chunk.set(Spaces::Cs([6i32, -20, 9]), 92.into());

        let chunk_packet = ReplyChunk {
            request_id: 560,
            chunk: chunk.clone(),
        };

        let sent_packet = AddressedPacket {
            caller_id: client_id,
            packet: Box::new(chunk_packet),
        };

        networker.send(sent_packet);

        let recved_packet = {
            let header = Header::sync_read(&mut stream).unwrap();
            let mut buf = vec![0u8; header.compressed_len as usize];

            stream.read_exact(&mut buf).unwrap();
            let decompressed_buf = {
                let mut decompressor = ZlibDecoder::new(&buf[..]);
                let mut buf = Vec::<u8>::new();
                decompressor.read_to_end(&mut buf).unwrap();
                buf
            };

            let id = u16::from_be_bytes(decompressed_buf[..2].try_into().unwrap());
            assert_eq!(id, ReplyChunk::ID);

            bincode::deserialize::<ReplyChunk>(&decompressed_buf[2..]).unwrap()
        };

        assert_eq!(recved_packet.request_id, 560);
        assert_eq!(recved_packet.chunk, chunk);
    }
}
