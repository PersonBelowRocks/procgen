use std::ops::Range;

use num_traits::PrimInt;

use crate::{block::BlockId, util::cast_vec3, volume::CubicVolume};

pub const CHUNK_SECTION_SIZE: usize = 16;

pub type ChunkSection = CubicVolume<BlockId, CHUNK_SECTION_SIZE>;

type IVec2 = na::Vector2<i32>;

pub struct Chunk {
    pub(super) pos: IVec2,
    pub(super) sections: Vec<Option<Box<ChunkSection>>>,
    default_id: BlockId,
    vertical_bounds: Range<i32>,
}

impl Chunk {
    pub fn try_new(pos: IVec2, max_y: i32, min_y: i32, default_id: BlockId) -> Option<Self> {
        let max_y = (max_y as i32 / CHUNK_SECTION_SIZE as i32) * CHUNK_SECTION_SIZE as i32;
        let min_y = (min_y as i32 / CHUNK_SECTION_SIZE as i32) * CHUNK_SECTION_SIZE as i32;

        if max_y <= min_y {
            return None;
        }

        let height = (max_y - min_y) as usize;
        if (height as usize) < CHUNK_SECTION_SIZE {
            return None;
        }

        let sections = vec![None; height as usize / CHUNK_SECTION_SIZE];

        Some(Self {
            pos,
            sections,
            default_id,
            vertical_bounds: min_y..max_y,
        })
    }

    pub fn pos(&self) -> IVec2 {
        self.pos
    }

    pub fn min_y(&self) -> i32 {
        self.vertical_bounds.start
    }

    pub fn max_y(&self) -> i32 {
        self.vertical_bounds.end
    }

    pub fn default_id(&self) -> BlockId {
        self.default_id
    }

    pub fn sections(&self) -> usize {
        self.sections.len()
    }

    #[inline]
    pub fn abs_height(&self) -> u32 {
        (self.sections.capacity() as u32) * (CHUNK_SECTION_SIZE as u32)
    }

    #[inline]
    pub(super) fn chunk_y_to_section_idx(&self, chunk_y: i32) -> usize {
        (chunk_y - self.min_y()) as usize / CHUNK_SECTION_SIZE
    }

    #[inline]
    pub(super) fn chunk_y_to_index_y(&self, chunk_y: i32) -> u32 {
        (chunk_y - self.min_y()) as u32
    }

    #[inline]
    pub(super) fn index_y_to_chunk_y(&self, index_y: u32) -> i32 {
        index_y as i32 + self.min_y()
    }

    #[inline]
    pub fn within_bounds<N: PrimInt>(&self, ws_position: na::Vector3<N>) -> bool {
        let chunk_corner_pos = na::vector![
            self.pos[0] * CHUNK_SECTION_SIZE as i32,
            0,
            self.pos[1] * CHUNK_SECTION_SIZE as i32
        ];
        let ws_position = cast_vec3::<i32, N>(ws_position).unwrap();

        let [x, y, z]: [i32; 3] = (ws_position - chunk_corner_pos).into();

        let mut within = (self.min_y()..self.max_y()).contains(&y);
        within &= (0..CHUNK_SECTION_SIZE as i32).contains(&x);
        within &= (0..CHUNK_SECTION_SIZE as i32).contains(&z);

        within
    }

    #[inline]
    pub fn within_bounds_cs<N: PrimInt>(&self, cs_position: na::Vector3<N>) -> bool {
        let [x, y, z]: [i32; 3] = cast_vec3::<i32, N>(cs_position).unwrap().into();

        let mut within = (self.min_y()..self.max_y()).contains(&y);
        within &= (0..CHUNK_SECTION_SIZE as i32).contains(&x);
        within &= (0..CHUNK_SECTION_SIZE as i32).contains(&z);

        within
    }
}
