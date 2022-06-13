use std::marker::PhantomData;

use crate::{
    block::BlockId,
    chunk::Chunk,
    generation::{FactoryParameters, GenerationArgs},
    runtime::util::{GeneratorId, RequestId},
};

pub(crate) trait DowncastPacket: dc::DowncastSync + Send + std::fmt::Debug {}

pub(crate) trait Packet: serde::Serialize + serde::de::DeserializeOwned {
    const ID: u16;
}

impl<P> DowncastPacket for P where P: Packet + dc::Downcast + Send + Sync + std::fmt::Debug {}

dc::impl_downcast!(DowncastPacket);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct GenerateChunk {
    pub(crate) request_id: RequestId,
    pub(crate) generator_id: GeneratorId,
    pub(crate) pos: na::Vector2<i32>,
}

impl GenerateChunk {
    pub fn args(&self) -> GenerationArgs {
        GenerationArgs { pos: self.pos }
    }
}

impl Packet for GenerateChunk {
    const ID: u16 = 0;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReplyChunk {
    pub(crate) request_id: RequestId,
    pub(crate) chunk: Chunk,
}

impl Packet for ReplyChunk {
    const ID: u16 = 1;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct AddGenerator {
    pub(crate) request_id: RequestId,
    pub(crate) name: String,
    pub(crate) min_height: i32,
    pub(crate) max_height: i32,
    pub(crate) default_id: BlockId,
}

impl AddGenerator {
    pub fn factory_params(&self) -> FactoryParameters<'_> {
        FactoryParameters {
            min_height: self.min_height,
            max_height: self.max_height,
            default: self.default_id,

            _future_noncopy_params: PhantomData,
        }
    }
}

impl Packet for AddGenerator {
    const ID: u16 = 2;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfirmGeneratorAddition {
    pub(crate) request_id: RequestId,
    pub(crate) generator_id: GeneratorId,
}

impl ConfirmGeneratorAddition {
    pub fn new(request_id: RequestId, generator_id: GeneratorId) -> Self {
        Self {
            request_id,
            generator_id,
        }
    }
}

impl Packet for ConfirmGeneratorAddition {
    const ID: u16 = 3;
}
