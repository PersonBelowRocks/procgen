use crate::chunk::Chunk;

pub trait Generator: Sync + Send {
    fn fill_chunk(&self, chunk: &mut Chunk);
}
