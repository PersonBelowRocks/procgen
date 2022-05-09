use std::{net::SocketAddrV4, sync::Arc};

use tokio::{io::BufReader, sync::RwLock};

use super::{compressor::PacketCompressor, protocol::DownstreamPacket, server::AsyncStream};

// This struct is generic so that we can use mock streams for testing and TCP streams in the actual program.
#[derive(Debug)]
pub(super) struct Connection<S>
where
    S: AsyncStream,
{
    stream: Arc<RwLock<BufReader<S>>>,
    compressor: Arc<PacketCompressor>,
    address: SocketAddrV4,
}

// Rust didn't want to derive clone for some reason so we gotta implement it ourselves.
impl<S> Clone for Connection<S>
where
    S: AsyncStream,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            compressor: self.compressor.clone(),
            address: self.address,
        }
    }
}

impl<S> Connection<S>
where
    S: AsyncStream,
{
    pub(super) fn new(address: SocketAddrV4, stream: S, compressor: Arc<PacketCompressor>) -> Self {
        let stream = BufReader::new(stream);

        Self {
            stream: Arc::new(RwLock::new(stream)),
            compressor,
            address,
        }
    }

    pub(super) fn address(&self) -> SocketAddrV4 {
        self.address
    }

    pub(super) fn can_write(&self) -> bool {
        self.stream.try_write().is_ok()
    }

    pub(super) async fn read_packet(&mut self) -> DownstreamPacket {
        todo!()
    }

    pub(super) async fn write_packet(&mut self) {
        todo!()
    }
}
