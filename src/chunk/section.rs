use vol::prelude::*;

use crate::{block::BlockId, util::IVec3};

use super::basic::CHUNK_SECTION_CORNER;

pub struct ChunkSection {
    default: BlockId,
    pos: IVec3, // TODO: should we bother giving sections a position? they're pretty much only used from within chunks anyways so we can do
    // transformations and stuff there
    volume: Option<HeapVolume<BlockId>>, // TODO: this volume is allocated on the heap which takes a while,
                                         // finish the StackVolume implementation in the "volume" crate and use that instead!
}

impl ChunkSection {
    pub fn new_uninitialized(default: BlockId, pos: IVec3) -> Self {
        Self {
            default,
            pos,
            volume: None,
        }
    }

    pub fn new_initialized(default: BlockId, pos: IVec3) -> Self {
        let mut new = Self::new_uninitialized(default, pos);
        new.initialize();
        new
    }

    #[inline]
    fn initialize(&mut self) {
        // Only do this if we're uninitialized so we avoid wiping any existing data.
        if !self.is_initialized() {
            self.volume = Some(HeapVolume::new(
                self.default,
                self.pos..(self.pos + CHUNK_SECTION_CORNER),
            ));
        }
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.volume.is_some()
    }
}

impl Volume for ChunkSection {
    type Item = BlockId;

    #[inline(always)]
    fn ls_get(&self, idx: [u64; 3]) -> Option<&Self::Item> {
        match self.volume {
            Some(ref vol) => vol.ls_get(idx),
            None => Some(&self.default),
        }
    }

    #[inline(always)]
    fn ls_get_mut(&mut self, idx: [u64; 3]) -> Option<&mut Self::Item> {
        match self.volume {
            Some(ref mut vol) => vol.ls_get_mut(idx),
            None => {
                // We're gonna initialize the section here for 2 reasons:
                // - We need to return a mutable reference which cannot be owned by the function, and returning a mutable reference
                //   to our own default BlockId (like Self::ls_get) would be incredibly dumb, because then the caller could mutate our default which could be
                //   reused in the future, leading to unpredictable behaviour and horrific bugs.
                //
                // - The caller may have called this function by calling Volume::swap, in which case they (probably) want to swap whatever is at this index.
                //   It's ergonomic and convenient for the caller if the section just initializes itself in that case, instead of panicking or
                //   returning something stupid (see above), eliminating the need for a bunch of safeguards and sanity checks.
                self.initialize();
                self.ls_get_mut(idx)
            }
        }
    }

    #[inline(always)]
    fn bounding_box(&self) -> BoundingBox {
        if let Some(ref vol) = self.volume {
            // We'll grab the existing bounding box if it exists due to performance reasons.
            // BoundingBox::new has some logic in it that we might wanna avoid running multiple times.
            vol.bounding_box()
        } else {
            BoundingBox::new(self.pos.into(), (self.pos + CHUNK_SECTION_CORNER).into())
        }
    }
}
