use vol::prelude::*;

use crate::{
    block::BlockId,
    generation::GenerationArgs,
    util::{IVec2, IVec3},
};

use super::section::ChunkSection;

/// X and Z dimensions of chunks (taken from Minecraft)
pub const CHUNK_SIZE: i32 = 16;

/// Corner of a chunk section.
/// This constant is here for ergonomics so you can do add it to a chunk section's position and get the position of the opposite corner.
pub const CHUNK_SECTION_CORNER: IVec3 = na::vector![CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE];

fn chunk_sections_for_height(height: i32) -> usize {
    debug_assert!(height >= 0); // we can't have chunks with a negative height

    ((height + CHUNK_SIZE - ((height - 1).rem_euclid(CHUNK_SIZE))) / CHUNK_SIZE) as usize
}

// TODO: implement Chunks
pub struct Chunk {
    sections: Vec<ChunkSection>, // TODO: maybe extract this into its own type?
    bounding_box: BoundingBox,
}

impl Chunk {
    pub fn new(default: BlockId, chunk_pos: IVec2, min_height: i32, max_height: i32) -> Self {
        let pos = chunk_pos * CHUNK_SIZE;

        let sections = {
            let capacity = chunk_sections_for_height((min_height - max_height).abs());
            let mut vec = Vec::with_capacity(capacity);

            for _ in 0..capacity {
                vec.push(ChunkSection::new_uninitialized(default));
            }

            debug_assert!(vec.len() == capacity);

            vec
        };

        let bounding_box = BoundingBox::new([pos.x, min_height, pos.y], [pos.x, max_height, pos.y]);

        Self {
            sections,
            bounding_box,
        }
    }

    #[inline]
    pub fn from_args(args: GenerationArgs) -> Self {
        Self::new(args.default, args.pos, args.min_height, args.max_height)
    }

    #[inline]
    fn get_chunk_section(&self, chunk_section_idx: usize) -> Option<&ChunkSection> {
        self.sections.get(chunk_section_idx)
    }

    #[inline]
    fn get_mut_chunk_section(&mut self, chunk_section_idx: usize) -> Option<&mut ChunkSection> {
        self.sections.get_mut(chunk_section_idx)
    }

    /// Chunkspace access
    #[inline]
    fn cs_get<Idx: VolumeIdx>(&self, idx: Idx) -> Option<&<Self as Volume>::Item> {
        let [x, y, z] = idx.array::<i64>()?;
        self.ls_get([x, y - self.bounding_box().min()[1], z])
    }

    /// Mutable chunkspace access
    #[inline]
    fn cs_get_mut<Idx: VolumeIdx>(&mut self, idx: Idx) -> Option<&mut <Self as Volume>::Item> {
        let [x, y, z] = idx.array::<i64>()?;
        self.ls_get_mut([x, y - self.bounding_box().min()[1], z])
    }
}

impl Volume for Chunk {
    type Item = BlockId;

    #[inline]
    fn ls_get<Idx: VolumeIdx>(&self, idx: Idx) -> Option<&Self::Item> {
        let [x, y, z] = idx.array::<i32>()?;

        // This is the height/index of the chunk section containing the position provided.
        let section_idx = y / CHUNK_SIZE;
        // This is the position within that section corresponding to the position provided
        // (i.e., in the section's localspace).
        let sectionspace_y = y % CHUNK_SIZE;

        let section = self.get_chunk_section(section_idx as usize)?;
        section.ls_get([x, sectionspace_y, z])
    }

    #[inline]
    fn ls_get_mut<Idx: VolumeIdx>(&mut self, idx: Idx) -> Option<&mut Self::Item> {
        let [x, y, z] = idx.array::<i32>()?;

        // This is the height/index of the chunk section containing the position provided.
        let section_idx = y / CHUNK_SIZE;
        // This is the position within that section corresponding to the position provided
        // (i.e., in the section's localspace).
        let sectionspace_y = y % CHUNK_SIZE;

        let section = self.get_mut_chunk_section(section_idx as usize)?;
        section.ls_get_mut([x, sectionspace_y, z])
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_indexing() {
        const DEFAULT_ID: BlockId = BlockId::new(10);

        let mut chunk = Chunk::new(DEFAULT_ID, na::vector![2, 2], -64, 320);

        assert_eq!(chunk.get(na::vector![3i32, 3, 8]), None);
        assert_eq!(
            chunk.get(na::vector![3i32, 3, 8] + (na::vector![2, 0, 2] * CHUNK_SIZE)),
            Some(&DEFAULT_ID)
        );

        assert_eq!(chunk.get(na::vector![u64::MAX, 0, 0]), None);
        assert_eq!(chunk.get(na::vector![-100i32, 50, -10]), None);

        assert_eq!(
            chunk.swap(
                na::vector![3i32, 3, 8] + (na::vector![2, 0, 2] * CHUNK_SIZE),
                BlockId::new(42)
            ),
            Some(DEFAULT_ID)
        );
        assert_eq!(
            chunk.get(na::vector![3i32, 3, 8] + (na::vector![2i32, 0, 2] * CHUNK_SIZE)),
            Some(&BlockId::new(42))
        );

        assert_eq!(
            chunk.cs_get(na::vector![3i32, 3, 8]),
            Some(&BlockId::new(42))
        );
    }
}
