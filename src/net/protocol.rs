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

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestRegisterGenerator {
    name: String,
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

pub trait Packet {
    const PACKET_ID: u32;

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

trait BincodePacket
where
    Self: Serialize + DeserializeOwned + Packet,
{
    const PACKET_ID: u32;

    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    fn from_bytes(bytes: Vec<u8>) -> Self {
        bincode::deserialize_from(bytes.as_slice()).unwrap()
    }
}

impl<'a, T> Packet for T
where
    T: BincodePacket,
{
    const PACKET_ID: u32 = <Self as BincodePacket>::PACKET_ID;

    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        <Self as BincodePacket>::to_bytes(self)
    }

    #[inline]
    fn from_bytes(bytes: Vec<u8>) -> Self {
        <Self as BincodePacket>::from_bytes(bytes)
    }
}

impl BincodePacket for RequestRegisterGenerator {
    const PACKET_ID: u32 = 0x01;
}

impl BincodePacket for RespondRegisterGenerator {
    const PACKET_ID: u32 = 0x02;
}

impl BincodePacket for RequestGenerateChunk {
    const PACKET_ID: u32 = 0x03;
}

impl BincodePacket for RespondGenerateChunk {
    const PACKET_ID: u32 = 0x04;
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum DownstreamPacket {
    RequestRegisterGenerator(RequestRegisterGenerator),
    RequestGenerateChunk(RequestGenerateChunk),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum UpstreamPacket {
    RespondRegisterGenerator(RespondRegisterGenerator),
    RespondGenerateChunk(RespondGenerateChunk),
}
