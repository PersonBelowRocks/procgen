#[allow(dead_code)]
mod access;
#[allow(dead_code)]
mod basic;
#[allow(dead_code)]
mod trait_impls;

mod serialization;

pub use access::{ChunkAccessError, ChunkAccessResult};
pub use basic::{Chunk, ChunkSection, IVec2, CHUNK_SECTION_SIZE};

#[cfg(test)]
mod tests {
    use crate::block::BlockId;

    use super::*;

    /// Produce an example chunk for testing purposes. This function is purely for ergonomics and the chunk is not special in any way.
    ///
    /// # Example
    /// ```
    /// extern crate nalgebra as na;
    ///
    /// let chunk = example_chunk();
    ///
    /// assert_eq!(320, chunk.max_y());
    /// assert_eq!(-64, chunk.min_y());
    /// assert_eq!(BlockId::from(0), chunk.default_id());
    /// assert_eq!(na::vector![2, 2], chunk.pos());
    /// assert_eq!(384, chunk.abs_height());
    /// ```
    fn example_chunk() -> Chunk {
        Chunk::try_new(na::vector![2, 2], 320, -64, BlockId::from(0)).unwrap()
    }

    #[test]
    fn basics() {
        let chunk = example_chunk();

        assert_eq!(384, chunk.abs_height());
        assert_eq!(-64, chunk.min_y());
        assert_eq!(320, chunk.max_y());

        // Chunk with weird min_y bounds (not divisible by CHUNK_SECTION_SIZE)
        let chunk = Chunk::try_new(na::vector![2, 2], 320, -62, BlockId::from(0)).unwrap();

        assert_eq!(368, chunk.abs_height());
        // Should be rounded down to nearest number divisible by CHUNK_SECTION_SIZE
        assert_eq!(-48, chunk.min_y());
        assert_eq!(320, chunk.max_y());

        // Chunk with weird min_y bounds (not divisible by CHUNK_SECTION_SIZE)
        let chunk = Chunk::try_new(na::vector![2, 2], 338, -64, BlockId::from(0)).unwrap();

        assert_eq!(400, chunk.abs_height());
        assert_eq!(-64, chunk.min_y());
        // Should be rounded up to nearest number divisible by CHUNK_SECTION_SIZE
        assert_eq!(336, chunk.max_y());
    }

    #[test]
    fn bounds_checks() {
        let chunk = example_chunk();

        assert!(chunk.within_bounds(na::vector![40, 200, 47]));

        // Testing bounds in worldspace...
        // Closest corner
        assert!(chunk.within_bounds(na::vector![32, -64, 32]));
        // Any closer and we're outside...
        assert!(!chunk.within_bounds(na::vector![31, -64, 32]));
        assert!(!chunk.within_bounds(na::vector![32, -65, 32]));
        assert!(!chunk.within_bounds(na::vector![32, -64, 31]));

        // Furthest corner
        assert!(chunk.within_bounds(na::vector![47, 319, 47]));
        // Any further and we're outside...
        assert!(!chunk.within_bounds(na::vector![48, 319, 47]));
        assert!(!chunk.within_bounds(na::vector![47, 320, 47]));
        assert!(!chunk.within_bounds(na::vector![47, 319, 48]));

        // Testing bounds in chunkspace...
        // Closest corner
        assert!(chunk.within_bounds_cs(na::vector![0, -64, 0]));
        // Any closer and we're outside...
        assert!(!chunk.within_bounds_cs(na::vector![-1, -64, 0]));
        assert!(!chunk.within_bounds_cs(na::vector![0, -65, 0]));
        assert!(!chunk.within_bounds_cs(na::vector![0, -64, -1]));

        // Furthest corner
        assert!(chunk.within_bounds_cs(na::vector![15, 319, 15]));
        // Any further and we're outside...
        assert!(!chunk.within_bounds_cs(na::vector![16, 319, 15]));
        assert!(!chunk.within_bounds_cs(na::vector![15, 320, 15]));
        assert!(!chunk.within_bounds_cs(na::vector![15, 319, 16]));
    }

    #[test]
    fn indexing() {
        let mut chunk = example_chunk();

        let example_idx = na::vector![10, -60, 10i32];

        chunk.set(example_idx, 10.into()).unwrap();

        assert_eq!(BlockId::from(10), chunk[example_idx]);
        assert_eq!(BlockId::from(0), chunk[na::vector![11, -60, 10i32]]);

        for x in 0..16 {
            for z in 0..16 {
                for y in -64..-48 {
                    let idx = na::vector![x, y, z];

                    if idx == example_idx {
                        assert_eq!(BlockId::from(10), chunk[idx]);
                    } else {
                        assert_eq!(BlockId::from(0), chunk[idx]);
                    }
                }
            }
        }
    }

    #[test]
    fn indexing_mut() {
        let mut chunk = example_chunk();

        let example_idx = na::vector![10, 200, 10i32];

        chunk.set(example_idx, 10.into()).unwrap();

        assert_eq!(BlockId::from(10), chunk[example_idx]);
        assert_eq!(BlockId::from(0), chunk[na::vector![11, 200, 10i32]]);

        chunk[example_idx] = BlockId::from(42);
        assert_eq!(BlockId::from(42), chunk[example_idx]);
    }

    #[test]
    #[should_panic]
    fn indexing_oob() {
        let chunk = example_chunk();

        let idx = na::vector![14, -100, 10i32];
        // This index should be out of bounds and we should panic when trying to access it.
        if !chunk.within_bounds_cs(idx) {
            let _ = chunk[idx];
        }
    }

    #[test]
    #[should_panic]
    fn indexing_uninitialized_section() {
        let chunk = example_chunk();

        let idx = na::vector![14, 10, 10i32];
        // This index should be within bounds, and we should panic when we access it (section not initialized).
        if chunk.within_bounds_cs(idx) {
            let _ = chunk[idx];
        }
    }

    #[test]
    #[should_panic]
    fn indexing_invalid_vec() {
        // This test sort of sucks because we can't really tell what went wrong if it panicked.
        // Specifically we want it to go wrong because the index vector is messed up,
        // but we can't check if that's the reason it failed or because the vector is (for example) OOB.
        // FIXME: pretty please?
        let chunk = example_chunk();

        let idx = na::vector![i32::MAX, 10, 10];
        // This index is really busted and we should panic when trying to use it (it should fail while casting).
        let _ = chunk[idx];
    }

    #[test]
    fn accessing() {
        let chunk = example_chunk();

        // This is out of bounds
        if !matches!(
            chunk.get(na::vector![16, 200, 10]),
            Err(ChunkAccessError::IndexVectorOutOfBounds)
        ) {
            panic!("expected to receive Err(ChunkAccessError::IndexVectorOutOfBounds)!")
        }

        // None of these indices should be out of bounds, so we expect to see UninitializedSection for all of them.
        for x in 0..16 {
            for z in 0..16 {
                for y in -64..320 {
                    if !matches!(
                        chunk.get(na::vector![x, y, z]),
                        Err(ChunkAccessError::UninitializedSection)
                    ) {
                        panic!("expected to receive Err(ChunkAccessError::UninitializedSection)!")
                    }
                }
            }
        }
    }

    #[test]
    fn accessing_mut() {
        let mut chunk = example_chunk();

        for x in 0..16 {
            for z in 0..16 {
                for y in -64..320 {
                    if !matches!(
                        chunk.set_manual(na::vector![x, y, z], 10.into()),
                        Err(ChunkAccessError::UninitializedSection)
                    ) {
                        panic!("expected to receive Err(ChunkAccessError::UninitializedSection)!")
                    }
                }
            }
        }
        let example_idx = na::vector![10, -54, 10i32];
        // chunk.set(...) should initialize the section.
        assert_eq!(BlockId::from(0), chunk.set(example_idx, 10.into()).unwrap());

        // Check that this section is initialized now.
        for x in 0..16 {
            for z in 0..16 {
                for y in -64..-48 {
                    let index = na::vector![x, y, z];
                    let id = chunk.get(index).unwrap();

                    if index == example_idx {
                        assert_eq!(&BlockId::from(10), id);
                    } else {
                        assert_eq!(&BlockId::from(0), id);
                    }
                }
            }
        }

        // Check that everything else is untouched.
        for x in 0..16 {
            for z in 0..16 {
                for y in -48..320 {
                    if !matches!(
                        chunk.set_manual(na::vector![x, y, z], 10.into()),
                        Err(ChunkAccessError::UninitializedSection)
                    ) {
                        panic!("expected to receive Err(ChunkAccessError::UninitializedSection)!")
                    }
                }
            }
        }

        // Initialize the rest of the sections
        for y in 1..chunk.sections() as u32 {
            assert_eq!(
                BlockId::from(0),
                chunk
                    .set(
                        na::vector![5, chunk.index_y_to_chunk_y(y * 16), 5],
                        BlockId::from(8)
                    )
                    .unwrap()
            );
        }
    }

    #[test]
    #[should_panic]
    fn accessing_invalid_vec() {
        let chunk = example_chunk();
        // Way too big!
        let _ = chunk.get(na::vector![u32::MAX, 0, 0]);
    }

    #[test]
    fn section_initialization() {
        let mut chunk = example_chunk();

        // Index way too big
        if !matches!(
            chunk.init_section(100),
            Err(ChunkAccessError::SectionIndexOutOfBounds)
        ) {
            panic!("expected section index 100 to be out of bounds for chunk");
        }

        if !matches!(chunk.init_section(6), Ok(_)) {
            panic!("expected section index 6 to be valid, in bounds index for chunk")
        }

        if !matches!(
            chunk.init_section(6),
            Err(ChunkAccessError::SectionAlreadyInitialized)
        ) {
            panic!("expected section 6 to already be initialized")
        }
    }
}
