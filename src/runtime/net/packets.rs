use std::marker::PhantomData;

use crate::{
    block::BlockId,
    chunk::Chunk,
    generation::{FactoryParameters, GenerationArgs},
    runtime::util::{GeneratorId, RequestId},
};

use super::DynPacket;

pub fn parse_dyn(buf: &[u8]) -> anyhow::Result<DynPacket> {
    let id = u16::from_be_bytes(
        buf.get(..2)
            .ok_or_else(|| anyhow::anyhow!("malformed packet ID"))?
            .try_into()?,
    );

    let buf = buf
        .get(2..)
        .ok_or_else(|| anyhow::anyhow!("packet is too short"))?;

    match id {
        GenerateChunk::ID => Ok(Box::new(bincode::deserialize::<GenerateChunk>(buf)?)),
        ReplyChunk::ID => Ok(Box::new(bincode::deserialize::<ReplyChunk>(buf)?)),
        AddGenerator::ID => Ok(Box::new(bincode::deserialize::<AddGenerator>(buf)?)),
        ConfirmGeneratorAddition::ID => Ok(Box::new(bincode::deserialize::<
            ConfirmGeneratorAddition,
        >(buf)?)),
        ProtocolError::ID => Ok(Box::new(bincode::deserialize::<ProtocolError>(buf)?)),

        _ => Err(anyhow::anyhow!("invalid packet ID")),
    }
}

pub trait DowncastPacket: dc::DowncastSync + Send + std::fmt::Debug {}

pub trait Packet: serde::Serialize + serde::de::DeserializeOwned {
    const ID: u16;

    fn bincode(&self) -> Vec<u8> {
        let mut buf = Self::ID.to_be_bytes().to_vec();
        buf.extend(bincode::serialize(self).unwrap());
        buf
    }
}

impl<P> DowncastPacket for P where P: Packet + dc::Downcast + Send + Sync + std::fmt::Debug {}

dc::impl_downcast!(DowncastPacket);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GenerateChunk {
    pub request_id: RequestId,
    pub generator_id: GeneratorId,
    pub pos: na::Vector2<i32>,
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
pub struct ReplyChunk {
    pub request_id: RequestId,
    pub chunk: Chunk,
}

impl Packet for ReplyChunk {
    const ID: u16 = 1;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddGenerator {
    pub request_id: RequestId,
    pub name: String,
    pub min_height: i32,
    pub max_height: i32,
    pub default_id: BlockId,
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
pub struct ConfirmGeneratorAddition {
    pub request_id: RequestId,
    pub generator_id: GeneratorId,
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ProtocolErrorKind {
    Other {
        details: String,
    },
    GeneratorNotFound {
        generator_id: GeneratorId,
        request_id: RequestId,
    },
    ChunkGenerationFailure {
        generator_id: GeneratorId,
        request_id: RequestId,
        details: String,
    },
    Terminated {
        details: String,
    },
}

// TODO: finish implementing this
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
    const ID: u16 = 4;
}
