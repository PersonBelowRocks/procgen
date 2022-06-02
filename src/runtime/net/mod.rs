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

#[derive(Clone)]
pub(crate) struct NetworkerHandle {
    inbound: Arc<Mutex<Receiver<DynPacket>>>,
    outbound: Sender<DynPacket>,
}

impl NetworkerHandle {
    fn new(rx_inbound: Receiver<DynPacket>, tx_outbound: Sender<DynPacket>) -> Self {
        Self {
            inbound: Arc::new(Mutex::new(rx_inbound)),
            outbound: tx_outbound,
        }
    }
}

#[derive(Clone)]
pub(self) struct InternalNetworkerHandle {
    inbound: Sender<DynPacket>,
    outbound: Arc<Mutex<Receiver<DynPacket>>>,
}

impl InternalNetworkerHandle {
    fn new(tx_inbound: Sender<DynPacket>, rx_outbound: Receiver<DynPacket>) -> Self {
        Self {
            inbound: tx_inbound,
            outbound: Arc::new(Mutex::new(rx_outbound)),
        }
    }

    fn send(&self, packet: DynPacket) {
        self.inbound.send(packet).unwrap();
    }

    fn receive(&self) -> Option<DynPacket> {
        self.outbound.lock().unwrap().try_recv().ok()
    }
}

fn make_handles() -> (NetworkerHandle, InternalNetworkerHandle) {
    let (tx_i, rx_i) = mpsc::channel::<DynPacket>();
    let (tx_o, rx_o) = mpsc::channel::<DynPacket>();

    (
        NetworkerHandle::new(rx_i, tx_o),
        InternalNetworkerHandle::new(tx_i, rx_o),
    )
}

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
