use crate::{volume::CubicVolume, block::BlockId};

pub const CHUNK_SECTION_SIZE: usize = 16;

pub type ChunkSection = CubicVolume<BlockId, CHUNK_SECTION_SIZE>;

type IVec2 = na::Vector2<i32>;

pub struct Chunk {
    pub(super) pos: IVec2,
    pub(super) sections: Vec<Option<Box<ChunkSection>>>
}

impl Chunk {
    pub fn try_new(pos: IVec2, height: u32) -> Option<Self> {
        if (height as usize) < CHUNK_SECTION_SIZE {
            return None
        }

        let mut sections = Vec::with_capacity(height as usize / CHUNK_SECTION_SIZE);
        sections.fill(None);

        Some(Self {
            pos,
            sections
        })
    }

    pub fn pos(&self) -> IVec2 {
        self.pos
    }

    pub fn height(&self) -> u32 {
        (self.sections.len() as u32) * (CHUNK_SECTION_SIZE as u32)
    }
}