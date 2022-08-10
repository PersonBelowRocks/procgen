use super::chunk::Chunk;
use crate::volumes::ChunkStorage;
use crate::{PositionStatus, Positioned, Unpositioned};
use serde::de::Visitor;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

impl Serialize for Chunk<Unpositioned> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.storage.serialize(serializer)
    }
}

const POSITION_KEY: u8 = 0;
const STORAGE_KEY: u8 = 1;

impl Serialize for Chunk<Positioned> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        map.serialize_entry(&POSITION_KEY, &self.pos.position())?;
        map.serialize_entry(&STORAGE_KEY, &self.storage)?;

        map.end()
    }
}

impl<'de> Deserialize<'de> for Chunk<Unpositioned> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let storage = ChunkStorage::deserialize(deserializer)?;

        Ok(Self {
            storage: Box::new(storage),
            pos: Unpositioned,
        })
    }
}

use serde::de::Error;

struct PositionedChunkVisitor;

impl<'de> Visitor<'de> for PositionedChunkVisitor {
    type Value = Chunk<Positioned>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("16x16x16 chunk with an associated position")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        if let Some((key, position)) = map.next_entry::<u8, na::Vector3<i64>>()? {
            if key != POSITION_KEY {
                return Err(A::Error::custom("position key was not 0"));
            }

            if let Some((key, storage)) = map.next_entry::<u8, ChunkStorage>()? {
                if key != STORAGE_KEY {
                    return Err(A::Error::custom("storage key was not 1"));
                }

                return Ok(Chunk {
                    pos: Positioned(position),
                    storage: Box::new(storage),
                });
            }
        }

        Err(A::Error::custom("malformed positioned chunk data"))
    }
}

impl<'de> Deserialize<'de> for Chunk<Positioned> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(PositionedChunkVisitor)
    }
}

#[cfg(test)]
mod tests {
    use volume::Volume;

    use super::*;

    #[test]
    fn parity_unpositioned() {
        let mut cs = Chunk::<Unpositioned>::new();
        assert!(cs.set([5, 5, 5].into(), 202.into()));

        let bincode_cs = bincode::serialize(&cs).unwrap();
        let deser_cs = bincode::deserialize::<Chunk<Unpositioned>>(&bincode_cs).unwrap();

        assert_eq!(deser_cs, cs);
    }

    #[test]
    fn parity_positioned() {
        let mut cs = Chunk::<Positioned>::new([10, 11, 12].into());
        assert!(cs.set([5, 5, 5].into(), 202.into()));

        let bincode_cs = bincode::serialize(&cs).unwrap();
        let deser_cs = bincode::deserialize::<Chunk<Positioned>>(&bincode_cs).unwrap();

        assert_eq!(deser_cs, cs);
    }
}
