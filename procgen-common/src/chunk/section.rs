use vol::builtins::*;

use crate::BlockId;

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
