use std::ops::{Index, IndexMut};

use num_traits::PrimInt;

use crate::{block::BlockId, util::cast_ivec3};

use super::basic::{Chunk, CHUNK_SECTION_SIZE};

impl<N: PrimInt + Copy> Index<na::Vector3<N>> for Chunk {
    type Output = BlockId;

    #[inline]
    fn index(&self, v: na::Vector3<N>) -> &Self::Output {
        let [x, y, z]: [i32; 3] = cast_ivec3::<i32, N>(v).unwrap().into();
        let section_idx = self.chunk_y_to_section_idx(y);
        let y = self.chunk_y_to_index_y(y);

        let section = self.sections[section_idx].as_ref().unwrap().as_ref();

        &section[na::vector![x, y as i32 % CHUNK_SECTION_SIZE as i32, z]]
    }
}

impl<N: PrimInt> IndexMut<na::Vector3<N>> for Chunk {
    #[inline]
    fn index_mut(&mut self, v: na::Vector3<N>) -> &mut Self::Output {
        let [x, y, z]: [i32; 3] = cast_ivec3::<i32, N>(v).unwrap().into();
        let section_idx = self.chunk_y_to_section_idx(y);
        let y = self.chunk_y_to_index_y(y);

        let section = self.sections[section_idx].as_mut().unwrap().as_mut();

        &mut section[na::vector![x, y as i32 % CHUNK_SECTION_SIZE as i32, z]]
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let initialized_sections = self
            .sections
            .iter()
            .map(|s| match s {
                Some(_) => "#",
                None => ".",
            })
            .fold(String::new(), |accum, item| accum + item);

        write!(formatter, "Chunk {{")?;
        write!(formatter, "   pos: {},", self.pos())?;
        write!(formatter, "   sections: {}", initialized_sections)?;
        write!(formatter, "   default_id: {:?}", self.default_id())?;
        write!(
            formatter,
            "   vertical_bounds: {:?}",
            self.min_y()..self.max_y()
        )
    }
}
