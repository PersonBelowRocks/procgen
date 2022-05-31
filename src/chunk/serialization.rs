use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize};
use vol::prelude::*;

use super::basic::CHUNK_SIZE;
use super::section::ChunkSection;
use super::Chunk;

impl Serialize for ChunkSection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // We're going to store our default block ID as the first element in the sequence, hence the +1.
        let seq_len = 1 + (CHUNK_SIZE as usize).pow(3);
        let mut ser_seq = serializer.serialize_seq(Some(seq_len))?;

        ser_seq.serialize_element(&self.default_id())?;

        for z in 0..CHUNK_SIZE as usize {
            for y in 0..CHUNK_SIZE as usize {
                for x in 0..CHUNK_SIZE as usize {
                    ser_seq.serialize_element(self.get([x, y, z]).unwrap())?;
                }
            }
        }

        ser_seq.end()
    }
}

impl<'de> Deserialize<'de> for ChunkSection
where
    for<'a> Self: Deserialize<'a>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl Serialize for Chunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let pos = {
            let [x, _, z] = self.bounding_box().min();
            na::vector![x, z]
        };

        let min_height = self.bounding_box().min()[1];
        let max_height = self.bounding_box().max()[1];

        let mut ser_map = serializer.serialize_map(Some(5))?;
        ser_map.serialize_entry("position", &pos)?;
        ser_map.serialize_entry("min_height", &min_height)?;
        ser_map.serialize_entry("max_height", &max_height)?;

        for (i, section) in self.sections.iter().enumerate() {
            todo!()
        }

        todo!()
    }
}

impl<'de> Deserialize<'de> for Chunk
where
    for<'a> Self: Deserialize<'a>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
