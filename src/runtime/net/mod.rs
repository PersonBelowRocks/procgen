use std::sync::mpsc::{self, Receiver, Sender};

/// Represents an incoming command/request from a client.
pub(crate) enum NetInbound {}

/// Represents an outbound command/request to a client.
pub(crate) enum NetOutbound {}

pub(crate) struct NetworkerHandle {
    inbound: Receiver<NetInbound>,
    outbound: Sender<NetOutbound>,
}

impl NetworkerHandle {
    fn new(rx_inbound: Receiver<NetInbound>, tx_outbound: Sender<NetOutbound>) -> Self {
        Self {
            inbound: rx_inbound,
            outbound: tx_outbound,
        }
    }
}

pub(self) struct InternalNetworkerHandle {
    inbound: Sender<NetInbound>,
    outbound: Receiver<NetOutbound>,
}

impl InternalNetworkerHandle {
    fn new(tx_inbound: Sender<NetInbound>, rx_outbound: Receiver<NetOutbound>) -> Self {
        Self {
            inbound: tx_inbound,
            outbound: rx_outbound,
        }
    }
}

fn make_handles() -> (NetworkerHandle, InternalNetworkerHandle) {
    let (tx_i, rx_i) = mpsc::channel::<NetInbound>();
    let (tx_o, rx_o) = mpsc::channel::<NetOutbound>();

    (
        NetworkerHandle::new(rx_i, tx_o),
        InternalNetworkerHandle::new(tx_i, rx_o),
    )
}

pub(crate) struct Networker {
    runtime: tokio::runtime::Runtime,
    handle: NetworkerHandle,
}

async fn run(o: Receiver<NetOutbound>, i: Sender<NetInbound>) -> ! {
    // TODO: this is essentially #[tokio::main] but we manually build the runtime and submit this as the "main" function to it.
    // this function should set up all the networking stuff and then diverge into just serving terrain data over TCP.

    loop {
        todo!()
    }
}
