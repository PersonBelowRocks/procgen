use num_traits::PrimInt;

use crate::{block::BlockId, util::cast_vec3};

use super::basic::{Chunk, ChunkSection, CHUNK_SECTION_SIZE};

#[derive(Debug, thiserror::Error)]
pub enum ChunkAccessError {
    #[error("attempted access to a chunk section which is in-bounds, but not initialized")]
    UninitializedSection,
    #[error("index vector points to position outside of chunk bounds")]
    IndexVectorOutOfBounds,
}

pub type ChunkAccessResult<T> = Result<T, ChunkAccessError>;

impl Chunk {
    #[inline]
    pub fn get<N: PrimInt>(&self, v: na::Vector3<N>) -> ChunkAccessResult<&BlockId> {
        if !self.within_bounds_cs(v) {
            return Err(ChunkAccessError::IndexVectorOutOfBounds);
        }

        let [x, y, z]: [i32; 3] = cast_vec3::<i32, N>(v).unwrap().into();
        let y = self.chunk_y_to_index_y(y);

        if let Some(section) = &self.sections[y as usize / CHUNK_SECTION_SIZE] {
            let section_y = y % CHUNK_SECTION_SIZE as u32;
            Ok(&section[na::vector![x, section_y as i32, z]])
        } else {
            Err(ChunkAccessError::UninitializedSection)
        }
    }

    #[inline]
    pub fn set_manual<N: PrimInt>(
        &mut self,
        v: na::Vector3<N>,
        id: BlockId,
    ) -> ChunkAccessResult<BlockId> {
        if !self.within_bounds_cs(v) {
            return Err(ChunkAccessError::IndexVectorOutOfBounds);
        }

        let [x, y, z]: [i32; 3] = cast_vec3::<i32, N>(v).unwrap().into();
        let y = self.chunk_y_to_index_y(y);
        let section_idx = y as usize / CHUNK_SECTION_SIZE;

        if let Some(section) = &mut self.sections[section_idx] {
            let section_y = y % CHUNK_SECTION_SIZE as u32;
            let slot = &mut section[na::vector![x, section_y as i32, z]];

            Ok(std::mem::replace(slot, id))
        } else {
            Err(ChunkAccessError::UninitializedSection)
        }
    }

    #[inline]
    pub fn set<N: PrimInt>(
        &mut self,
        v: na::Vector3<N>,
        id: BlockId,
    ) -> ChunkAccessResult<BlockId> {
        if !self.within_bounds_cs(v) {
            return Err(ChunkAccessError::IndexVectorOutOfBounds);
        }

        match self.set_manual(v, id) {
            Ok(old_id) => Ok(old_id),
            Err(error) => {
                if matches!(error, ChunkAccessError::UninitializedSection) {
                    let [x, y, z]: [i32; 3] = cast_vec3::<i32, N>(v).unwrap().into();

                    let y = self.chunk_y_to_index_y(y);
                    let section_idx = y as usize / CHUNK_SECTION_SIZE;

                    self.sections[section_idx] = Some(Box::new(ChunkSection::default()));

                    let section_y = y % CHUNK_SECTION_SIZE as u32;
                    let slot = &mut self.sections[section_idx].as_mut().unwrap()
                        [na::vector![x, section_y as i32, z]];

                    Ok(std::mem::replace(slot, id))
                } else {
                    Err(error)
                }
            }
        }
    }
}
