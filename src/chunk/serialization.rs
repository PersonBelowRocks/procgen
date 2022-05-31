use serde::de::Visitor;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize};
use vol::prelude::*;

use crate::block::BlockId;

use super::basic::{chunk_sections_for_height, CHUNK_SIZE};
use super::section::ChunkSection;
use super::Chunk;

impl Serialize for ChunkSection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let seq_len = {
            2 + if self.is_initialized() {
                (CHUNK_SIZE as usize).pow(3)
            } else {
                0
            }
        };
        let mut ser_seq = serializer.serialize_seq(Some(seq_len))?;

        ser_seq.serialize_element(&self.default_id())?;
        ser_seq.serialize_element(&self.is_initialized())?;

        if self.is_initialized() {
            for z in 0..CHUNK_SIZE as usize {
                for y in 0..CHUNK_SIZE as usize {
                    for x in 0..CHUNK_SIZE as usize {
                        ser_seq.serialize_element(self.get([x, y, z]).unwrap())?;
                    }
                }
            }
        }

        ser_seq.end()
    }
}

struct ChunkSectionVisitor;

use serde::de::Error;

impl<'de> Visitor<'de> for ChunkSectionVisitor {
    type Value = ChunkSection;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a 16x16x16 chunk section")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let default_id = seq.next_element::<BlockId>()?.ok_or_else(|| {
            A::Error::custom("sequence was too short and did not contain the default block ID")
        })?;
        let initialized = seq.next_element::<bool>()?.ok_or_else(|| {
            A::Error::custom(
                "sequence was too short and did not contain the section's initialization status",
            )
        })?;

        if !initialized {
            return Ok(ChunkSection::new_uninitialized(default_id));
        }

        let mut section = ChunkSection::new_initialized(default_id);

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let voxel = seq
                        .next_element::<BlockId>()?
                        .ok_or_else(|| A::Error::custom("voxel sequence terminated prematurely"))?;

                    section.set([x, y, z], voxel)
                }
            }
        }

        if seq.next_element::<BlockId>()?.is_some() {
            Err(A::Error::custom("sequence was too long!"))
        } else {
            Ok(section)
        }
    }
}

impl<'de> Deserialize<'de> for ChunkSection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChunkSectionVisitor)
    }
}

impl Serialize for Chunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let pos = {
            let [x, _, z] = self.bounding_box().min();
            na::vector![x as i32, z as i32] / CHUNK_SIZE
        };

        let min_height = self.bounding_box().min()[1] as i32;
        let max_height = self.bounding_box().max()[1] as i32;

        let seq_len = 3 + self.sections.len();

        let mut ser_seq = serializer.serialize_seq(Some(seq_len))?;

        ser_seq.serialize_element(&pos)?;
        ser_seq.serialize_element(&min_height)?;
        ser_seq.serialize_element(&max_height)?;

        for section in self.sections.iter() {
            ser_seq.serialize_element(section)?;
        }

        ser_seq.end()
    }
}

struct ChunkVisitor;

impl<'de> Visitor<'de> for ChunkVisitor {
    type Value = Chunk;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a chunk's position, min height, max height, and a sequence containing its chunk sections")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let pos = seq.next_element::<na::Vector2<i32>>()?.ok_or_else(|| {
            A::Error::custom("sequence was too short and did not contain the chunk's position")
        })?;
        let min_height = seq.next_element::<i32>()?.ok_or_else(|| {
            A::Error::custom("sequence was too short and did not contain the chunk's min height")
        })?;
        let max_height = seq.next_element::<i32>()?.ok_or_else(|| {
            A::Error::custom("sequence was too short and did not contain the chunk's max height")
        })?;

        let n_sections = chunk_sections_for_height((min_height - max_height).abs());

        let mut sections = Vec::<ChunkSection>::with_capacity(n_sections);
        for _ in 0..n_sections {
            sections.push(seq.next_element::<ChunkSection>()?.ok_or_else(|| {
                A::Error::custom("sequence was too short and did not contain all sections")
            })?);
        }

        if seq.next_element::<ChunkSection>()?.is_some() {
            Err(A::Error::custom(format!("sequence was longer than expected given the specified chunk height (min: {}, max: {})", min_height, max_height)))
        } else {
            let mut empty_chunk = Chunk::new(sections[0].default_id(), pos, min_height, max_height);
            empty_chunk.sections = sections;
            Ok(empty_chunk)
        }
    }
}

impl<'de> Deserialize<'de> for Chunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChunkVisitor)
    }
}

// TODO: tests!
#[cfg(test)]
mod tests {
    use crate::chunk::basic::Spaces;

    use super::*;

    #[test]
    fn init_chunk_section_bincode_mirror() {
        let mut cs = ChunkSection::new_initialized(BlockId::new(50));
        cs.set([8i32, 8, 8], BlockId::new(12));
        cs.set([10i32, 4, 7], BlockId::new(61));

        let cs_copy: ChunkSection =
            bincode::deserialize(&bincode::serialize(&cs).unwrap()).unwrap();

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

        let cs_copy: ChunkSection =
            bincode::deserialize(&bincode::serialize(&cs).unwrap()).unwrap();

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
}
