use std::net::SocketAddrV4;

use tokio::io::BufReader;

use super::{
    compressor::{PacketCompressor, PacketReadingError},
    protocol::{DownstreamSuite, Packet, UpstreamSuite},
    server::AsyncStream,
};

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub(super) enum ConnectionError {
    #[error("Internal connection error: {0}")]
    Internal(String),
    #[error("Client lied or provided incorrect information about packet's compression sizes: {0}")]
    ClientCompressionLie(String),
    #[error("Could not decode packet sent by client, likely malformed: {0}")]
    MalformedPacket(String),
}

pub(super) type ConnectionResult<T> = Result<T, ConnectionError>;

// This struct is generic so that we can use mock streams for testing and TCP streams in the actual program.
#[derive(Debug)]
pub(super) struct Connection<S>
where
    S: AsyncStream,
{
    stream: BufReader<S>,
    compressor: PacketCompressor,
    address: SocketAddrV4,
}

impl<S> Connection<S>
where
    S: AsyncStream + Unpin,
{
    pub(super) fn new(address: SocketAddrV4, stream: S, compressor: PacketCompressor) -> Self {
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

    pub(super) async fn read_packet(&mut self) -> ConnectionResult<UpstreamSuite> {
        // A lot of stuff might happen here which is not useful to the caller,
        // so we map internal errors to more relevant and high-level ones
        // while still keeping the low-level debug information so that we can log it.

        // Currently (at the time of writing) there's not a whole lot of interesting mapping going on
        // but there will definitely be more in the future so this setup/boilerplate is nice.
        let packet_bytes = self
            .compressor
            .read_packet(&mut self.stream)
            .await
            .map_err(|err| {
                if let Some(packet_error) = err.downcast_ref::<PacketReadingError>() {
                    if matches!(
                        packet_error,
                        PacketReadingError::BadDecompressedLength(_, _)
                            | PacketReadingError::MismatchedPacketLengths(_, _)
                    ) {
                        return ConnectionError::ClientCompressionLie(packet_error.to_string());
                    }
                }

                ConnectionError::Internal(err.to_string())
            })?;
        UpstreamSuite::from_bytes(packet_bytes)
            .map_err(|err| ConnectionError::MalformedPacket(err.to_string()))
    }

    pub(super) async fn send_packet(&mut self, packet: DownstreamSuite) -> ConnectionResult<()> {
        let packet_bytes = packet.to_bytes();

        // See comment in Connection::read_packet
        self.compressor
            .write_packet(&mut self.stream, packet_bytes)
            .await
            .map_err(|err| ConnectionError::Internal(err.to_string()))
    }
}
