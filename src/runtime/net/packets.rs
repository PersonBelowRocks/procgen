use crate::chunk::Chunk;

pub(crate) trait Packet: dc::Downcast {
    fn id(&self) -> u16;
}

dc::impl_downcast!(Packet);

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct GenerateChunk {
    request_id: u32,
    generator_id: u32,
    pos: na::Vector2<i32>
}

impl Packet for GenerateChunk {
    fn id(&self) -> u16 {
        0
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ReplyChunk {
    request_id: u32,
    chunk: Chunk,
}

impl Packet for ReplyChunk {
    fn id(&self) -> u16 {
        1
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct AddGenerator {
    name: String
}

impl Packet for AddGenerator {
    fn id(&self) -> u16 {
        2
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfirmGeneratorAddition {
    generator_id: u32
}

impl Packet for ConfirmGeneratorAddition {
    fn id(&self) -> u16 {
        3
    }
}