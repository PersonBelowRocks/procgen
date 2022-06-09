use vol::builtins::*;

use crate::block::BlockId;

use super::basic::CHUNK_SIZE;

const CHUNK_SIZE_USIZE: usize = CHUNK_SIZE as usize;

type CubicVolume<const N: usize, T> = StackVolume<N, N, N, T>;
type ChunkSectionStorage = CubicVolume<CHUNK_SIZE_USIZE, BlockId>;

/// A 16x16x16 cube of voxels/blocks.
#[derive(Clone)]
pub struct ChunkSection {
    default: BlockId,
    volume: Option<ChunkSectionStorage>,
}

impl ChunkSection {
    #[inline]
    pub fn new_uninitialized(default: BlockId) -> Self {
        Self {
            default,
            volume: None,
        }
    }

    #[inline]
    pub fn new_initialized(default: BlockId) -> Self {
        let mut new = Self::new_uninitialized(default);
        new.initialize();
        new
    }

    #[inline]
    fn initialize(&mut self) {
        // Only do this if we're uninitialized so we avoid wiping any existing data.
        if !self.is_initialized() {
            self.volume = Some(ChunkSectionStorage::filled(self.default));
        }
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.volume.is_some()
    }

    #[inline]
    pub fn default_id(&self) -> BlockId {
        self.default
    }
}

impl std::cmp::PartialEq for ChunkSection {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if self.get([x, y, z]) != other.get([x, y, z]) {
                        return false;
                    }
                }
            }
        }

        true
    }
}

impl<Idx: VolumeIdx> VolumeAccess<Idx> for ChunkSection {
    #[inline]
    fn get(this: &Self, idx: Idx) -> Option<&Self::Item> {
        match this.volume {
            Some(ref v) => v.get(idx),
            None => {
                if this.contains(idx) {
                    Some(&this.default)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    fn set(this: &mut Self, idx: Idx, item: Self::Item) {
        if !this.is_initialized() {
            this.initialize();
            this.volume.as_mut().unwrap().set(idx, item);
        } else {
            this.volume.as_mut().unwrap().set(idx, item);
        }
    }

    #[inline]
    fn swap(this: &mut Self, idx: Idx, item: Self::Item) -> Option<Self::Item> {
        if !this.is_initialized() {
            this.initialize();
            this.volume.as_mut().unwrap().swap(idx, item)
        } else {
            this.volume.as_mut().unwrap().swap(idx, item)
        }
    }

    #[inline]
    fn contains(_this: &Self, idx: Idx) -> bool {
        if let Some([x, y, z]) = idx.array::<usize>() {
            x < CHUNK_SIZE_USIZE && y < CHUNK_SIZE_USIZE && z < CHUNK_SIZE_USIZE
        } else {
            false
        }
    }
}

impl Volume for ChunkSection {
    type Item = BlockId;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_section_indexing() {
        const DEFAULT_ID: BlockId = BlockId::new(5);

        let mut section = ChunkSection::new_uninitialized(DEFAULT_ID);

        assert_eq!(section.get([7i32, 7, 7]), Some(&DEFAULT_ID));
        assert_eq!(section.get([0i32, 0, 0]), Some(&DEFAULT_ID));
        assert_eq!(section.get([16i32, 16, 16]), None);
        assert_eq!(section.get([-1i32, -1, -1]), None);

        assert_eq!(
            section.swap([7i32, 7, 7], BlockId::new(42)),
            Some(DEFAULT_ID)
        );
        assert_eq!(section.get([7i32, 7, 7]), Some(&BlockId::new(42)));
    }

    #[test]
    fn chunk_section_bounds() {
        const DEFAULT_ID: BlockId = BlockId::new(5);

        let section = ChunkSection::new_uninitialized(DEFAULT_ID);

        assert!(section.contains([0i32, 0, 0]));
        assert!(section.contains([15i32, 15, 15]));

        assert!(!section.contains([-1i32, -1, -1]));
        assert!(!section.contains([16i32, 16, 16]));
    }
}
