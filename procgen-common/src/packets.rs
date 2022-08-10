use std::{
    io::{self, Read},
    mem::size_of,
};

use crate::{Chunk, IVec3, Parameters, Positioned};

use crate::RequestId;

pub trait DowncastPacket: dc::DowncastSync + Send + std::fmt::Debug {}

pub trait Packet: serde::Serialize + serde::de::DeserializeOwned + Clone {
    const ID: u16;

    fn to_bincode(&self) -> Result<PacketBuffer, PacketBufferError> {
        PacketBuffer::from_packet(self)
    }

    fn from_bincode(buf: &PacketBuffer) -> Result<Self, PacketBufferError> {
        buf.to_packet()
    }
}

impl<P> DowncastPacket for P where P: Packet + dc::Downcast + Send + Sync + std::fmt::Debug {}

dc::impl_downcast!(DowncastPacket);

#[derive(te::Error, Debug)]
pub enum PacketBufferError {
    #[error("Packet was too short and did not contain an ID")]
    PacketTooShort,
    #[error("Error when serializing or deserializing buffer: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("Attempted to deserialize buffer with ID {0} into a packet with ID {1}")]
    MismatchedPacketId(u16, u16),
    #[error("IO error when producing a buffer from a stream: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Debug, Hash, PartialEq)]
pub struct PacketBuffer {
    inner: Vec<u8>,
}

impl PacketBuffer {
    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, PacketBufferError> {
        let mut buf = Vec::<u8>::new();

        // We only allow valid packet data, so there must be enough bytes to produce an ID.
        if reader.read_to_end(&mut buf)? < size_of::<u16>() {
            return Err(PacketBufferError::PacketTooShort);
        }

        Ok(Self { inner: buf })
    }

    pub fn id(&self) -> u16 {
        u16::from_be_bytes(self.inner[..size_of::<u16>()].try_into().unwrap())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn from_packet<P: Packet>(packet: &P) -> Result<Self, PacketBufferError> {
        let mut buf = P::ID.to_be_bytes().to_vec();
        buf.extend(bincode::serialize(packet)?);

        Ok(Self { inner: buf })
    }

    pub fn to_packet<P: Packet>(&self) -> Result<P, PacketBufferError> {
        if self.id() != P::ID {
            return Err(PacketBufferError::MismatchedPacketId(self.id(), P::ID));
        }
        let packet = bincode::deserialize::<P>(&self.inner[size_of::<u16>()..])?;
        Ok(packet)
    }
}

impl AsRef<[u8]> for PacketBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ProtocolErrorKind {
    Other {
        details: String,
    },
    GeneratorNotFound {
        generator_name: String,
        request_id: RequestId,
    },
    GenerationError {
        generator_name: String,
        request_id: RequestId,
        details: String,
    },
    FactoryError {
        generator_name: String,
        request_id: RequestId,
        details: String,
    },
    Terminated {
        details: String,
    },
}

// TODO: finish implementing this
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolError {
    pub kind: ProtocolErrorKind,
    pub fatal: bool,
}

impl ProtocolError {
    pub fn gentle(kind: ProtocolErrorKind) -> Self {
        Self { kind, fatal: false }
    }

    pub fn fatal(kind: ProtocolErrorKind) -> Self {
        Self { kind, fatal: true }
    }
}

impl Packet for ProtocolError {
    const ID: u16 = 0;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GenerateRegion {
    pub request_id: RequestId,
    pub bounds: std::ops::Range<IVec3>,
    pub params: Parameters,
}

impl Packet for GenerateRegion {
    const ID: u16 = 1;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateBrush {
    pub request_id: RequestId,
    pub pos: IVec3,
    pub params: Parameters,
}

impl Packet for GenerateBrush {
    const ID: u16 = 2;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FinishRequest {
    pub request_id: RequestId,
}

impl Packet for FinishRequest {
    const ID: u16 = 3;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoxelData {
    pub request_id: RequestId,
    pub data: Chunk<Positioned>,
}

impl Packet for VoxelData {
    const ID: u16 = 4;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AckRequest {
    pub request_id: RequestId,
    pub info: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestGenerators {
    pub request_id: RequestId,
}

impl Packet for RequestGenerators {
    const ID: u16 = 6;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListGenerators {
    pub request_id: RequestId,
    pub generators: Vec<String>,
}

impl Packet for ListGenerators {
    const ID: u16 = 7;
}
