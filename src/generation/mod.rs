pub trait ChunkGenerator: Send + Sync + DynChunkGenerator {
    const NAME: &'static str;
    fn generate(&self, chunk: &mut ()) -> anyhow::Result<()>;
}

pub trait DynChunkGenerator: Send + Sync {
    // TODO: figure this little guy out
    fn generate(&self, chunk: &mut ()) -> anyhow::Result<()>;
}

impl<T: ChunkGenerator> DynChunkGenerator for T {
    #[inline(always)]
    fn generate(&self, chunk: &mut ()) -> anyhow::Result<()> {
        <Self as ChunkGenerator>::generate(self, chunk)
    }
}

pub type DynGenVTable = Box<dyn DynChunkGenerator>;
