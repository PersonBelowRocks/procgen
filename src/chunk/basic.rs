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
}

impl Volume for Chunk {
    type Item = BlockId;

    #[inline]
    fn ls_get(&self, idx: [u64; 3]) -> Option<&Self::Item> {
        // This is basically the index of the section in self.sections
        let section_cy = (idx[1] as usize) / CHUNK_SIZE as usize;
        // This is the y position within the section. If idx[1] was for example 20,
        // it would index into the section at self.sections[1] and get a block at Y=4 in that section's localspace.
        // (so sectionspace_y == 4)
        let sectionspace_y = idx[1] % CHUNK_SIZE as u64;

        let section = self.get_chunk_section(section_cy)?;
        section.ls_get([idx[0], sectionspace_y, idx[2]])
    }

    #[inline]
    fn ls_get_mut(&mut self, idx: [u64; 3]) -> Option<&mut Self::Item> {
        // See Self::ls_get above for an explanation of these.
        let section_cy = (idx[1] as usize) / CHUNK_SIZE as usize;
        let sectionspace_y = idx[1] % CHUNK_SIZE as u64;

        let section = self.get_mut_chunk_section(section_cy)?;
        section.ls_get_mut([idx[0], sectionspace_y, idx[2]])
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }
}
