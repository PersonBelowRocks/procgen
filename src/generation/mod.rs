use crate::{block::BlockId, chunk::Chunk};

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct GenerationArgs {
    max_height: i32,
    min_height: i32,
    default: BlockId,
    // TODO: this should have a seed field too for RNG
}

pub trait ChunkGenerator: Send + Sync + DynChunkGenerator {
    const NAME: &'static str;
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()>;
}

pub trait DynChunkGenerator: Send + Sync {
    // TODO: figure this little guy out
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()>;
}

impl<T: ChunkGenerator> DynChunkGenerator for T {
    #[inline(always)]
    fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()> {
        <Self as ChunkGenerator>::generate(self, chunk)
    }
}

pub type DynGenVTable = Box<dyn DynChunkGenerator>;
