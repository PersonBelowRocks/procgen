use std::sync::Arc;

use ecs::prelude::*;

use crate::{
    chunk::Chunk,
    generation::{ChunkGenerator, DynGenVTable},
};

#[derive(Component, Clone)]
pub struct Generator(Arc<DynGenVTable>);

impl<G: ChunkGenerator + 'static> From<G> for Generator {
    fn from(generator: G) -> Self {
        Self(Arc::new(Box::new(generator)))
    }
}

impl Generator {
    /// Fill the given `chunk` using the generator.
    pub fn generate(&self, chunk: &mut Chunk) -> anyhow::Result<()> {
        self.0.generate(chunk)
    }
}

#[derive(Component, Hash, PartialEq)]
pub struct GeneratorName(String);

impl GeneratorName {
    pub fn equals(&self, s: &str) -> bool {
        self.0 == s
    }
}

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
