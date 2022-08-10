use vol::prelude::*;

use super::chunk::Chunk;
use crate::{BlockId, IVec3, PositionStatus, Positioned, Unpositioned, VoxelSlot};

pub trait Boundness {}

#[derive(Copy, Clone, Debug)]
pub struct Unbounded;
impl Boundness for Unbounded {}

#[derive(Copy, Clone, Debug)]
pub struct Bounded(BoundingBox);
impl Boundness for Bounded {}

pub struct VoxelVolume<B: Boundness> {
    bounded: B,
    sections: hb::HashMap<IVec3, Chunk<Unpositioned>>,
}

impl<B: Boundness> VoxelVolume<B> {
    pub fn into_chunks(self) -> ChunkIter {
        ChunkIter {
            iter: self.sections.into_iter(),
        }
    }

    pub fn add_chunk(&mut self, chunk: Chunk<Positioned>) {
        self.sections
            .insert(chunk.pos.position(), chunk.to_unpositioned());
    }
}

impl VoxelVolume<Unbounded> {
    pub fn new() -> Self {
        Self {
            bounded: Unbounded,
            sections: Default::default(),
        }
    }
}

pub struct ChunkIter {
    iter: hb::hash_map::IntoIter<IVec3, Chunk<Unpositioned>>,
}

impl Iterator for ChunkIter {
    type Item = Chunk<Positioned>;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, chunk) = self.iter.next()?;
        Some(chunk.to_positioned(pos))
    }
}

impl Default for VoxelVolume<Unbounded> {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelVolume<Bounded> {
    pub fn new(bounds: BoundingBox) -> Self {
        Self {
            bounded: Bounded(bounds),
            sections: Default::default(),
        }
    }
}

impl Volume for VoxelVolume<Unbounded> {
    type Input = BlockId;
    type Output = Option<BlockId>;

    #[inline]
    fn set(&mut self, idx: Idx, item: Self::Input) -> bool {
        use hb::hash_map::Entry;

        let size = Chunk::<Unpositioned>::SIZE;

        let section_pos = idx / size;
        let voxel_pos: IVec3 = [idx.x % size, idx.y % size, idx.z % size].into();

        match self.sections.entry(section_pos) {
            Entry::Occupied(mut entry) => entry.get_mut().set(voxel_pos, item),
            Entry::Vacant(entry) => entry
                .insert(Chunk::<Unpositioned>::new())
                .set(voxel_pos, item),
        }
    }

    #[inline]
    fn get(&self, idx: Idx) -> Self::Output {
        let size = Chunk::<Unpositioned>::SIZE;

        let section_pos = idx / size;
        let voxel_pos: IVec3 = [idx.x % size, idx.y % size, idx.z % size].into();

        self.sections
            .get(&section_pos)
            .and_then(|section| Option::from(section.get(voxel_pos)))
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        (na::vector![i64::MIN, i64::MIN, i64::MIN]..na::vector![i64::MAX, i64::MAX, i64::MAX])
            .into()
    }
}

impl Volume for VoxelVolume<Bounded> {
    type Input = BlockId;
    type Output = VoxelSlot;

    #[inline]
    fn set(&mut self, idx: Idx, item: Self::Input) -> bool {
        use hb::hash_map::Entry;

        if !self.bounded.0.contains(idx) {
            return false;
        }

        let size = Chunk::<Unpositioned>::SIZE;

        let section_pos = idx / size;
        let voxel_pos: IVec3 = [idx.x % size, idx.y % size, idx.z % size].into();

        match self.sections.entry(section_pos) {
            Entry::Occupied(mut entry) => entry.get_mut().set(voxel_pos, item),
            Entry::Vacant(entry) => entry
                .insert(Chunk::<Unpositioned>::new())
                .set(voxel_pos, item),
        }
    }

    #[inline]
    fn get(&self, idx: Idx) -> Self::Output {
        if !self.bounded.0.contains(idx) {
            return VoxelSlot::OutOfBounds;
        }

        let size = Chunk::<Unpositioned>::SIZE;

        let section_pos = idx / size;
        let voxel_pos: IVec3 = [idx.x % size, idx.y % size, idx.z % size].into();

        match self.sections.get(&section_pos) {
            Some(section) => section.get(voxel_pos),
            None => VoxelSlot::Empty,
        }
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        self.bounded.0
    }
}
