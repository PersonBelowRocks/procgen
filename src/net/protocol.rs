use enum_ordinalize::Ordinalize;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestRegisterGenerator<'a> {
    name: &'a str,
    generator_id: GeneratorId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RespondRegisterGenerator {
    response_code: GeneratorResponseCode,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestGenerateChunk {
    pos: IVec2,
    request_id: RequestId,
    generator_id: GeneratorId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RespondGenerateChunk {
    request_id: RequestId,
    chunk: Chunk,
}

pub trait Packet<'a> {
    const PACKET_ID: u32;

    fn to_bytes(&self, version: ProtocolVersion) -> Vec<u8>;
    fn from_bytes(bytes: &'a [u8], version: ProtocolVersion) -> Self;
}

trait BincodePacket<'a>
where
    Self: Serialize + Deserialize<'a> + Packet<'a>,
{
    const PACKET_ID: u32;

    #[inline]
    fn to_bytes(&self, _version: ProtocolVersion) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    fn from_bytes(bytes: &'a [u8], _version: ProtocolVersion) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

impl<'a, T> Packet<'a> for T
where
    T: BincodePacket<'a>,
{
    const PACKET_ID: u32 = <Self as BincodePacket<'a>>::PACKET_ID;

    #[inline]
    fn to_bytes(&self, version: ProtocolVersion) -> Vec<u8> {
        <Self as BincodePacket<'a>>::to_bytes(self, version)
    }

    #[inline]
    fn from_bytes(bytes: &'a [u8], version: ProtocolVersion) -> Self {
        <Self as BincodePacket<'a>>::from_bytes(bytes, version)
    }
}

impl<'a> BincodePacket<'a> for RequestRegisterGenerator<'a> {
    const PACKET_ID: u32 = 0x01;
}

impl<'a> BincodePacket<'a> for RespondRegisterGenerator {
    const PACKET_ID: u32 = 0x02;
}

impl<'a> BincodePacket<'a> for RequestGenerateChunk {
    const PACKET_ID: u32 = 0x03;
}

impl<'a> BincodePacket<'a> for RespondGenerateChunk {
    const PACKET_ID: u32 = 0x04;
}
