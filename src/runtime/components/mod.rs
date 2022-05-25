use ecs::prelude::*;

use crate::{
    chunk::Chunk,
    generation::{ChunkGenerator, DynGenVTable},
};

#[derive(Component)]
pub struct Generator(DynGenVTable);

impl<G: ChunkGenerator + 'static> From<G> for Generator {
    fn from(generator: G) -> Self {
        Self(Box::new(generator))
    }
}

impl Generator {
    pub fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()> {
        self.0.generate(chunk)
    }
}

#[derive(Component, Hash, PartialEq)]
pub struct GeneratorName(String);

impl std::fmt::Display for GeneratorName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for GeneratorName {
    fn from(s: String) -> Self {
        Self(s)
    }
}
