use std::{net::SocketAddrV4, sync::Arc};

use tokio::{io::BufReader, sync::RwLockWriteGuard};

use super::{
    compressor::PacketCompressor,
    protocol::{DownstreamPacket, Packet, RequestGenerateChunk, RequestRegisterGenerator},
    server::AsyncStream,
};

// This struct is generic so that we can use mock streams for testing and TCP streams in the actual program.
#[derive(Debug)]
pub(super) struct Connection<S>
where
    S: AsyncStream,
{
    stream: BufReader<S>,
    compressor: Arc<PacketCompressor>,
    address: SocketAddrV4,
}

impl<S> Connection<S>
where
    S: AsyncStream + Unpin,
{
    pub(super) fn new(address: SocketAddrV4, stream: S, compressor: Arc<PacketCompressor>) -> Self {
        let stream = BufReader::new(stream);

        Self {
            stream,
            compressor,
            address,
        }
    }

    pub(super) fn address(&self) -> SocketAddrV4 {
        self.address
    }

    pub(super) async fn read_packet(&mut self) -> anyhow::Result<DownstreamPacket> {
        todo!()
    }

    pub(super) async fn write_packet(&mut self) {
        todo!()
    }
}
