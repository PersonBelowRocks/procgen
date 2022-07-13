use volume::Volume;

use crate::{
    chunk::{
        basic::{Spaces, CHUNK_SIZE},
        section::ChunkSection,
    },
    BlockId, Chunk,
};

#[test]
fn chunk_indexing() {
    const DEFAULT_ID: BlockId = BlockId::new(10);

    let mut chunk = Chunk::new(DEFAULT_ID, na::vector![2, 2], -64, 320);

    assert_eq!(chunk.get(Spaces::Ws(na::vector![3i32, 3, 8])), None);
    assert_eq!(
        chunk.get(Spaces::Ws(
            na::vector![3i32, 3, 8] + (na::vector![2, 0, 2] * CHUNK_SIZE)
        )),
        Some(&DEFAULT_ID)
    );

    assert_eq!(chunk.get(na::vector![u64::MAX, 0, 0]), None);
    assert_eq!(chunk.get(na::vector![-100i32, 50, -10]), None);

    assert_eq!(
        chunk.swap(
            Spaces::Ws(na::vector![3i32, 3, 8] + (na::vector![2, 0, 2] * CHUNK_SIZE)),
            BlockId::new(42)
        ),
        Some(DEFAULT_ID)
    );
    assert_eq!(
        chunk.get(Spaces::Ws(
            na::vector![3i32, 3, 8] + (na::vector![2i32, 0, 2] * CHUNK_SIZE)
        )),
        Some(&BlockId::new(42))
    );

    assert_eq!(
        chunk.get(Spaces::Cs(na::vector![3i32, 3, 8])),
        Some(&BlockId::new(42))
    );
}

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

#[test]
fn init_chunk_section_bincode_mirror() {
    let mut cs = ChunkSection::new_initialized(BlockId::new(50));
    cs.set([8i32, 8, 8], BlockId::new(12));
    cs.set([10i32, 4, 7], BlockId::new(61));

    let cs_copy: ChunkSection = bincode::deserialize(&bincode::serialize(&cs).unwrap()).unwrap();

    assert_eq!(cs_copy.is_initialized(), cs.is_initialized());
    assert_eq!(cs_copy.default_id(), cs.default_id());

    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = [x, y, z];

                assert_eq!(cs.get(idx), cs_copy.get(idx));
            }
        }
    }
}

#[test]
fn uninit_chunk_section_bincode_mirror() {
    let cs = ChunkSection::new_uninitialized(BlockId::new(50));

    let cs_copy: ChunkSection = bincode::deserialize(&bincode::serialize(&cs).unwrap()).unwrap();

    assert_eq!(cs_copy.is_initialized(), cs.is_initialized());
    assert_eq!(cs_copy.default_id(), cs.default_id());
}

#[test]
fn chunk_bincode_mirror() {
    let mut chunk = Chunk::new(BlockId::new(10), na::vector![4, 4], -64, 320);

    chunk.set(Spaces::Cs([8i32, 0, 8]), BlockId::new(42));
    chunk.set(Spaces::Cs([5i32, -43, 9]), BlockId::new(12));

    let chunk_copy: Chunk = bincode::deserialize(&bincode::serialize(&chunk).unwrap()).unwrap();

    assert_eq!(chunk_copy.default_id(), chunk.default_id());
    assert_eq!(chunk_copy.bounding_box(), chunk.bounding_box());

    let min = chunk.bounding_box().min();
    let max = chunk.bounding_box().max();

    for z in min[2]..max[2] {
        for y in min[1]..max[1] {
            for x in min[0]..max[0] {
                let idx = [x, y, z];

                assert_eq!(chunk_copy.get(idx), chunk.get(idx));
            }
        }
    }
}
