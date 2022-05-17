use std::ops::Range;

use crate::{block::BlockId, chunk::Chunk, net::protocol::GeneratorId};

pub trait ChunkGenerator
where
    Self: Sync + Send,
{
    fn bounds(&self) -> Range<i32>;

    fn default_id(&self) -> BlockId {
        BlockId::new(0)
    }
    fn fill_chunk(&self, chunk: &mut Chunk);
}

pub trait HasGeneratorId {
    const GENERATOR_ID: GeneratorId;
}
