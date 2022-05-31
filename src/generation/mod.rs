use crate::{block::BlockId, chunk::Chunk, util::IVec2};

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct GenerationArgs {
    pub max_height: i32,
    pub min_height: i32,
    pub default: BlockId,
    pub pos: IVec2,
    // TODO: this should have a seed field too for RNG
}

pub trait ChunkGenerator: Send + Sync + DynChunkGenerator {
    /// The name of this generator. Must be unique or you'll suffer.
    const NAME: &'static str;

    /// Fill the given `chunk` using the generator.
    /// Implementors can honestly do whatever they feel like here with the chunk, this is THE terrain generation function.
    /// Erroring as an implementor will (probably) just result in the error getting logged and the server continuing, so any error handling must be done
    /// manually within the function.
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()>;
}

pub trait DynChunkGenerator: Send + Sync {
    /// Fill the given `chunk` using the generator.
    /// Implementors can honestly do whatever they feel like here with the chunk, this is THE terrain generation function.
    /// Erroring as an implementor will (probably) just result in the error getting logged and the server continuing, so any error handling must be done
    /// manually within the function.
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()>;
}

impl<T: ChunkGenerator> DynChunkGenerator for T {
    #[inline]
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()> {
        <Self as ChunkGenerator>::generate(self, chunk)
    }
}

pub type DynGenVTable = Box<dyn DynChunkGenerator>;
