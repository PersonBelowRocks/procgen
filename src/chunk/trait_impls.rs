use std::ops::{Index, IndexMut};

use num_traits::PrimInt;

use crate::{block::BlockId, util::cast_vec3};

use super::basic::{Chunk, CHUNK_SECTION_SIZE};

impl<N: PrimInt + Copy> Index<na::Vector3<N>> for Chunk {
    type Output = BlockId;

    #[inline]
    fn index(&self, v: na::Vector3<N>) -> &Self::Output {
        let [x, y, z]: [u32; 3] = cast_vec3::<u32, N>(v).unwrap().into();

        let section = self.sections[(y / CHUNK_SECTION_SIZE as u32) as usize]
            .as_ref()
            .unwrap()
            .as_ref();

        &section[na::vector![x, y % CHUNK_SECTION_SIZE as u32, z]]
    }
}

impl<N: PrimInt> IndexMut<na::Vector3<N>> for Chunk {
    #[inline]
    fn index_mut(&mut self, v: na::Vector3<N>) -> &mut Self::Output {
        let [x, y, z]: [u32; 3] = cast_vec3::<u32, N>(v).unwrap().into();

        let section = self.sections[(y / CHUNK_SECTION_SIZE as u32) as usize]
            .as_mut()
            .unwrap()
            .as_mut();

        &mut section[na::vector![x, y % CHUNK_SECTION_SIZE as u32, z]]
    }
}
