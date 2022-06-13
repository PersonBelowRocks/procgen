use std::marker::PhantomData;

use crate::{block::BlockId, chunk::Chunk, util::IVec2};

// TODO: rename this to something with parameters (instead of args) for consistency
#[derive(Debug, Copy, Clone)]
pub struct GenerationArgs {
    pub pos: IVec2,
}

#[allow(dead_code)]
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

pub trait DynGeneratorFactory: Send {
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator>;
}

impl<T: GeneratorFactory> DynGeneratorFactory for T {
    #[inline]
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator> {
        Box::new(<T as GeneratorFactory>::create(self, params))
    }
}

pub trait GeneratorFactory: Send + 'static {
    type Generator: ChunkGenerator;

    fn create(&self, params: FactoryParameters<'_>) -> Self::Generator;
}

pub trait ChunkGenerator: Send + Sync + DynChunkGenerator {
    /// The name of this generator. Must be unique or you'll suffer.
    const NAME: &'static str;

    type Factory: GeneratorFactory<Generator = Self> + 'static;

    /// Fill the given `chunk` using the generator.
    /// Implementors can honestly do whatever they feel like here with the chunk, this is THE terrain generation function.
    /// Erroring as an implementor will (probably) just result in the error getting logged and the server continuing, so any error handling must be done
    /// manually within the function.
    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk>;

    fn factory() -> Self::Factory;
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
