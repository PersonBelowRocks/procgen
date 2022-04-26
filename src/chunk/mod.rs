#[allow(dead_code)]
mod access;
#[allow(dead_code)]
mod basic;
#[allow(dead_code)]
mod trait_impls;

pub use access::{ChunkAccessError, ChunkAccessResult};
pub use basic::{Chunk, ChunkSection, CHUNK_SECTION_SIZE};

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: more tests!

    #[test]
    fn basics() {
        let chunk = Chunk::try_new(na::vector![2, 2], 320, -64).unwrap();

        assert_eq!(384, chunk.abs_height());
        assert_eq!(-64, chunk.min_y());
        assert_eq!(320, chunk.max_y());

        // Chunk with weird min_y bounds (not divisible by CHUNK_SECTION_SIZE)
        let chunk = Chunk::try_new(na::vector![2, 2], 320, -62).unwrap();

        assert_eq!(368, chunk.abs_height());
        // Should be rounded down to nearest number divisible by CHUNK_SECTION_SIZE
        assert_eq!(-48, chunk.min_y());
        assert_eq!(320, chunk.max_y());

        // Chunk with weird min_y bounds (not divisible by CHUNK_SECTION_SIZE)
        let chunk = Chunk::try_new(na::vector![2, 2], 338, -64).unwrap();

        assert_eq!(400, chunk.abs_height());
        assert_eq!(-64, chunk.min_y());
        // Should be rounded up to nearest number divisible by CHUNK_SECTION_SIZE
        assert_eq!(336, chunk.max_y());
    }

    #[test]
    fn bounds_checks() {
        let chunk = Chunk::try_new(na::vector![2, 2], 320, -64).unwrap();

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
    fn indexing() {}

    #[test]
    #[should_panic]
    fn indexing_oob() {
        panic!()
    }

    #[test]
    #[should_panic]
    fn indexing_invalid_vec() {
        panic!()
    }

    #[test]
    fn accessing() {
        let chunk = Chunk::try_new(na::vector![2, 2], 320, -64).unwrap();

        // This is out of bounds
        if !matches!(
            chunk.get(na::vector![16, 200, 10]),
            Err(ChunkAccessError::IndexVectorOutOfBounds)
        ) {
            panic!("expected to receive Err(ChunkAccessError::IndexVectorOutOfBounds)!")
        }

        // We're gonna check that every index produces a ChunkAccessError::UninitializedSection error
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
    #[should_panic]
    fn accessing_invalid_vec() {
        panic!()
    }
}
