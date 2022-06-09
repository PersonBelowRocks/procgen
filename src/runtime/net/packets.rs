use crate::chunk::Chunk;

pub(crate) trait DowncastPacket: dc::DowncastSync + Send + std::fmt::Debug {}

pub(crate) trait Packet: serde::Serialize + serde::de::DeserializeOwned {
    const ID: u16;
}

impl<P> DowncastPacket for P where P: Packet + dc::Downcast + Send + Sync + std::fmt::Debug {}

dc::impl_downcast!(DowncastPacket);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct GenerateChunk {
    pub(super) request_id: u32,
    pub(super) generator_id: u32,
    pub(super) pos: na::Vector2<i32>,
}

impl Packet for GenerateChunk {
    const ID: u16 = 0;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReplyChunk {
    pub(super) request_id: u32,
    pub(super) chunk: Chunk,
}

impl Packet for ReplyChunk {
    const ID: u16 = 1;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct AddGenerator {
    pub(super) request_id: u32,
    pub(super) name: String,
}

impl Packet for AddGenerator {
    const ID: u16 = 2;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfirmGeneratorAddition {
    pub(super) request_id: u32,
    pub(super) generator_id: u32,
}

impl Packet for ConfirmGeneratorAddition {
    const ID: u16 = 3;
}
