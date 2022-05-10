use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::chunk::{Chunk, IVec2};

#[allow(dead_code)]
#[non_exhaustive]
pub enum ProtocolVersion {
    V1,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::V1
    }
}

pub type GeneratorId = u32;
pub type RequestId = u32;

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratorResponseCode(u8);

pub trait Packet
where
    Self: Sized,
{
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self>;
}

trait BincodePacket
where
    Self: Serialize + DeserializeOwned,
{
    #[inline]
    fn to_bincode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    fn from_bincode(bytes: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize_from(bytes.as_slice())?)
    }
}

impl<'a, T> Packet for T
where
    T: BincodePacket,
{
    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bincode()
    }

    #[inline]
    fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        Self::from_bincode(bytes).into()
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub enum DownstreamSuite {
    RequestRegisterGenerator {
        name: String,
        generator_id: GeneratorId,
    },
    RequestGenerateChunk {
        pos: IVec2,
        request_id: RequestId,
        generator_id: GeneratorId,
    },
}

impl BincodePacket for DownstreamSuite {}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub enum UpstreamSuite {
    RespondRegisterGenerator {
        response_code: GeneratorResponseCode,
    },
    RespondGenerateChunk {
        request_id: RequestId,
        chunk: Chunk,
    },
}

impl BincodePacket for UpstreamSuite {}
