use num_traits::PrimInt;

use crate::block::BlockId;

use super::basic::Chunk;

#[derive(Debug, thiserror::Error)]
pub enum ChunkAccessError {
    #[error("attempted access to a chunk section which is in-bounds, but not initialized")]
    UninitializedSection,
    #[error("index vector points to position outside of chunk bounds")]
    IndexVectorOutOfBounds,
}

pub type ChunkAccessResult<T> = Result<T, ChunkAccessError>;

impl Chunk {
    pub fn get<N: PrimInt>(&self, v: na::Vector3<N>) -> ChunkAccessResult<&BlockId> {
        // let [x, y, z] = cast_vec3::<u32, N>(v).into().unwrap();
        
        todo!()
    }

    pub fn set<N: PrimInt>(&mut self, v: na::Vector3<N>, id: BlockId) -> ChunkAccessResult<()> {
        todo!()
    }
}