use std::marker::PhantomData;

use crate::{block::BlockId, chunk::Chunk, util::IVec2};

// TODO: rename this to something with parameters (instead of args) for consistency
#[derive(Debug, Copy, Clone)]
pub struct GenerationArgs {
    pub pos: IVec2,
}

#[derive(Copy, Clone)]
pub struct FactoryParameters<'a> {
    pub(crate) max_height: i32,
    pub(crate) min_height: i32,
    pub(crate) default: BlockId,

    // this is here because in the future we're gonna want more complex and lengthy parameters, in which case we don't want to copy them.
    // it's always a hassle to convert code with plenty of copying into non-copy code after it's written, so we'll write it like this from the start.
    pub(crate) _future_noncopy_params: PhantomData<&'a [u8]>,
    // TODO: this should have a seed field too for RNG
}

pub trait GeneratorFactory: Send {
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator>;
}

pub trait ChunkGenerator: Send + Sync + DynChunkGenerator + GeneratorFactory {
    /// The name of this generator. Must be unique or you'll suffer.
    const NAME: &'static str;

    /// Fill the given `chunk` using the generator.
    /// Implementors can honestly do whatever they feel like here with the chunk, this is THE terrain generation function.
    /// Erroring as an implementor will (probably) just result in the error getting logged and the server continuing, so any error handling must be done
    /// manually within the function.
    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk>;

    fn create(params: FactoryParameters<'_>) -> Self;
}

pub trait DynChunkGenerator: Send + Sync {
    /// Fill the given `chunk` using the generator.
    /// Implementors can honestly do whatever they feel like here with the chunk, this is THE terrain generation function.
    /// Erroring as an implementor will (probably) just result in the error getting logged and the server continuing, so any error handling must be done
    /// manually within the function.
    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk>;
}

impl<T: ChunkGenerator> DynChunkGenerator for T {
    #[inline]
    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk> {
        <Self as ChunkGenerator>::generate(self, args)
    }
}

impl<T: ChunkGenerator + 'static> GeneratorFactory for T {
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator> {
        Box::new(<Self as ChunkGenerator>::create(params))
    }
}
