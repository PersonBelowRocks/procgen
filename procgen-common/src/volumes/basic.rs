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
        let bounds: BoundingBox = {
            let min = bounds.min();
            let max = bounds.max(); // + na::vector![1, 1, 1];
            (min..max).into()
        };

        Self {
            bounded: Bounded(bounds),
            sections: Default::default(),
        }
    }
}

fn to_section_pos(idx: IVec3) -> IVec3 {
    let size = Chunk::<Positioned>::SIZE;

    [
        idx.x.div_euclid(size),
        idx.y.div_euclid(size),
        idx.z.div_euclid(size),
    ]
    .into()
}

fn to_voxel_pos(idx: IVec3) -> IVec3 {
    let size = Chunk::<Positioned>::SIZE;

    [
        idx.x.rem_euclid(size),
        idx.y.rem_euclid(size),
        idx.z.rem_euclid(size),
    ]
    .into()
}

impl Volume for VoxelVolume<Unbounded> {
    type Input = BlockId;
    type Output = Option<BlockId>;

    #[inline]
    fn set(&mut self, idx: Idx, item: Self::Input) -> bool {
        use hb::hash_map::Entry;

        let section_pos = to_section_pos(idx);
        let voxel_pos = to_voxel_pos(idx);

        match self.sections.entry(section_pos) {
            Entry::Occupied(mut entry) => entry.get_mut().set(voxel_pos, item),
            Entry::Vacant(entry) => entry
                .insert(Chunk::<Unpositioned>::new())
                .set(voxel_pos, item),
        }
    }

    #[inline]
    fn get(&self, idx: Idx) -> Self::Output {
        let section_pos = to_section_pos(idx);
        let voxel_pos = to_voxel_pos(idx);

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

        if !self.bounding_box().contains(idx) {
            return false;
        }

        let section_pos = to_section_pos(idx);
        let voxel_pos = to_voxel_pos(idx);

        match self.sections.entry(section_pos) {
            Entry::Occupied(mut entry) => entry.get_mut().set(voxel_pos, item),
            Entry::Vacant(entry) => entry
                .insert(Chunk::<Unpositioned>::new())
                .set(voxel_pos, item),
        }
    }

    #[inline]
    fn get(&self, idx: Idx) -> Self::Output {
        if !self.bounding_box().contains(idx) {
            return VoxelSlot::OutOfBounds;
        }

        let section_pos = to_section_pos(idx);
        let voxel_pos = to_voxel_pos(idx);

        match self.sections.get(&section_pos) {
            Some(section) => section.get(voxel_pos),
            None => VoxelSlot::Empty,
        }
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        let min = self.bounded.0.min();
        let max = self.bounded.0.max() + na::vector![1, 1, 1];

        (min..max).into()
    }
}

#[cfg(test)]
mod tests {
    use volume::BoundingBox;
    use volume::Volume;

    use crate::Bounded;
    use crate::VoxelVolume;

    #[test]
    fn bounded_voxel_volume() {
        let bounds: BoundingBox = (na::vector![-560, 136, 209]..na::vector![-619, 153, 169]).into();
        let mut volume = VoxelVolume::<Bounded>::new(bounds);

        let min = bounds.min();
        let max = bounds.max();

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    let pos = na::vector![x, y, z];
                    assert!(volume.set(pos, 1.into()));
                }
            }
        }
    }

    #[test]
    fn bounded_voxel_volume_single_chunk() {
        let bounds: BoundingBox = (na::vector![-561, 66, 112]..na::vector![-576, 79, 127]).into();
        let mut volume = VoxelVolume::<Bounded>::new(bounds);

        let min = bounds.min();
        let max = bounds.max();

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    let pos = na::vector![x, y, z];
                    assert!(volume.set(pos, 1.into()));
                }
            }
        }

        let chunks = volume.into_chunks().collect::<Vec<_>>();

        for chunk in chunks.iter() {
            dbg!(chunk);
        }

        assert_eq!(chunks.len(), 1);
    }
}
