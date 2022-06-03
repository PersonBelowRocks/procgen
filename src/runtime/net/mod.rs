mod internal;
pub mod packets;

use std::{
    net::SocketAddrV4,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

use flate2::Compression;

use self::packets::DowncastPacket;

type DynPacket = Box<dyn DowncastPacket>;
type ChannelData = AddressedPacket;

#[derive(Debug)]
struct AddressedPacket {
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
            runtime: Arc::new(tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()),
            handle: None,
        }
    }

    pub fn run(&mut self, params: Params) -> NetworkerHandle {
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

        self.handle.clone().unwrap()
    }

    pub fn handle(&self) -> NetworkerHandle {
        self.handle.clone().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, net::TcpStream, time::Duration};

    use flate2::write::ZlibEncoder;

    use super::{internal::Header, packets::Packet, *};

    #[test]
    fn end_to_end_test_networker() {
        let mut networker = Networker::new();

        let params = Params {
            addr: "0.0.0.0:33445".parse().unwrap(),
            compression: Compression::best(),
        };

        let handle = networker.run(params);

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

        match handle
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
}
