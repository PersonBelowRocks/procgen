use std::fmt;

use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::ChunkSection;

impl Serialize for ChunkSection {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let seq_length = Self::capacity();
        let mut sequence = serializer.serialize_seq(Some(seq_length))?;

        for idx in self.iter_indices() {
            sequence.serialize_element(&self[idx])?;
        }

        sequence.end()
    }
}

struct ChunkSectionVisitor;

impl<'de> Visitor<'de> for ChunkSectionVisitor {
    type Value = ChunkSection;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("(this part is not filled in yet)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut out = ChunkSection::default();

        for (c, idx) in out.iter_indices().enumerate() {
            let val = seq
                .next_element::<u32>()?
                .ok_or_else(|| serde::de::Error::invalid_length(c, &self))?;

            out[idx] = val.into();
        }

        Ok(out)
    }
}

impl<'de> Deserialize<'de> for ChunkSection {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(ChunkSectionVisitor)
    }
}

#[cfg(test)]
mod test {

    // TODO: better/more serialization testing
    use crate::{block::BlockId, chunk::Chunk};

    use super::*;

    #[test]
    fn chunk_section_parity() {
        let mut example_section = ChunkSection::new_filled(BlockId::from(10));
        for (n, idx) in example_section.iter_indices().enumerate() {
            example_section[idx] = BlockId::from(n as u32);
        }

        let bc_serialized = bincode::serialize(&example_section).unwrap();
        let example_section_deserialized: ChunkSection =
            bincode::deserialize(&bc_serialized[..]).unwrap();

        assert!(example_section == example_section_deserialized);
    }

    #[test]
    fn chunk_parity() {
        let mut example = Chunk::try_new(na::vector![2, 2], 320, -64, BlockId::from(0)).unwrap();
        example
            .set(na::vector![10, 10, 4i32], BlockId::from(42))
            .unwrap();
        example
            .set(na::vector![10, 251, 9i32], BlockId::from(41))
            .unwrap();

        let bc_serialized = bincode::serialize(&example).unwrap();
        let example_section_deserialized: Chunk = bincode::deserialize(&bc_serialized[..]).unwrap();

        assert!(example == example_section_deserialized);
    }
}
