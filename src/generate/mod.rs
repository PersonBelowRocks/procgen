use crate::chunk::Chunk;

pub trait Generator {
    type Options;

    fn fill_chunk(&self, chunk: &mut Chunk);
    fn set_options(&mut self, options: &Self::Options);
}
