use common::generation::{FactoryParameters, GenerationArgs};
use common::Chunk;

pub trait DynGeneratorFactory: Send + Sync {
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator>;
}

impl<T: GeneratorFactory> DynGeneratorFactory for T {
    #[inline]
    fn create(&self, params: FactoryParameters<'_>) -> Box<dyn DynChunkGenerator> {
        Box::new(<T as GeneratorFactory>::create(self, params))
    }
}

pub trait GeneratorFactory: Send + Sync + 'static {
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
