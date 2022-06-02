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
    runtime: tokio::runtime::Runtime,
    handle: Option<NetworkerHandle>,
}

impl Networker {
    pub fn new() -> Self {
        Self {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            handle: None,
        }
    }

    pub fn run(&mut self, params: Params) -> NetworkerHandle {
        let (external, internal) = make_handles();
        self.handle = Some(external);

        self.runtime.spawn(internal::run(params, internal));

        self.handle.clone().unwrap()
    }

    pub fn handle(&self) -> NetworkerHandle {
        self.handle.clone().unwrap()
    }
}
