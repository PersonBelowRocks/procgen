use num_traits::PrimInt;

use crate::{block::BlockId, util::cast_ivec3};

use super::basic::{Chunk, ChunkSection, CHUNK_SECTION_SIZE};

#[derive(Debug, thiserror::Error)]
pub enum ChunkAccessError {
    #[error("attempted access to a chunk section which is in-bounds, but not initialized")]
    UninitializedSection,
    #[error("index vector points to position outside of chunk bounds")]
    IndexVectorOutOfBounds,
    #[error("section index out of bounds")]
    SectionIndexOutOfBounds,
    #[error("section already initialized")]
    SectionAlreadyInitialized,
}

pub type ChunkAccessResult<T> = Result<T, ChunkAccessError>;

impl Chunk {
    pub fn init_section(&mut self, section_idx: usize) -> ChunkAccessResult<&mut ChunkSection> {
        let default_id = self.default_id();

        let slot = self
            .sections
            .get_mut(section_idx)
            .ok_or(ChunkAccessError::SectionIndexOutOfBounds)?;

        if slot.is_some() {
            return Err(ChunkAccessError::SectionAlreadyInitialized);
        }

        *slot = Some(Box::new(ChunkSection::new_filled(default_id)));

        Ok(self
            .sections
            .get_mut(section_idx)
            .unwrap()
            .as_mut()
            .unwrap())
    }

    #[inline]
    pub fn get<N: PrimInt>(&self, v: na::Vector3<N>) -> ChunkAccessResult<&BlockId> {
        if !self.within_bounds_cs(v) {
            return Err(ChunkAccessError::IndexVectorOutOfBounds);
        }

        let [x, y, z]: [i32; 3] = cast_ivec3::<i32, N>(v).unwrap().into();
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

        let [x, y, z]: [i32; 3] = cast_ivec3::<i32, N>(v).unwrap().into();
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
                    let [x, y, z]: [i32; 3] = cast_ivec3::<i32, N>(v).unwrap().into();

                    let y = self.chunk_y_to_index_y(y);
                    let section_idx = y as usize / CHUNK_SECTION_SIZE;

                    let section = self.init_section(section_idx).unwrap();

                    let section_y = y % CHUNK_SECTION_SIZE as u32;
                    let slot = &mut section[na::vector![x, section_y as i32, z]];

                    Ok(std::mem::replace(slot, id))
                } else {
                    Err(error)
                }
            }
        }
    }
}
