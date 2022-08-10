use std::any::type_name;

use serde::de::DeserializeOwned;
use serde::de::Error;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserialize;
use serde::Serialize;
use vol::prelude::*;

use crate::BlockId;
use crate::IVec3;

const SIZE: i64 = 16;
const SIZE_USIZE: usize = SIZE as usize;

type CubicVolume<const N: usize, T> = [[[T; N]; N]; N];

#[derive(Clone, PartialEq, Eq)]
pub(in crate::volumes) struct ChunkStorage(CubicVolume<SIZE_USIZE, Option<BlockId>>);

impl ChunkStorage {
    pub fn empty() -> Self {
        Self([[[None; SIZE_USIZE]; SIZE_USIZE]; SIZE_USIZE])
    }
}

impl serde::Serialize for ChunkStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(SIZE_USIZE.pow(3)))?;

        for z in 0..SIZE {
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let item = self.0.get(na::vector![x, y, z]).unwrap();
                    seq.serialize_element(&item)?;
                }
            }
        }

        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for ChunkStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChunkStorageVisitor)
    }
}

pub(in crate::volumes) struct ChunkStorageVisitor;

impl<'de> Visitor<'de> for ChunkStorageVisitor {
    type Value = ChunkStorage;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("voxel sequence representing data stored in a chunk")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut storage = ChunkStorage::empty();

        for z in 0..SIZE {
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let voxel = seq
                        .next_element::<Option<BlockId>>()?
                        .ok_or_else(|| A::Error::custom("voxel sequence terminated prematurely"))?;

                    storage.0.set(na::vector![x, y, z], voxel);
                }
            }
        }

        if seq.next_element::<BlockId>()?.is_some() {
            Err(A::Error::custom("sequence was too long!"))
        } else {
            Ok(storage)
        }
    }
}

/// A 16x16x16 cube of voxels/blocks.
#[derive(Clone, PartialEq, Eq)]
pub struct Chunk<P: PositionStatus> {
    // this is boxed so we don't constantly overflow stacks when working with chunks, especially with async stuff
    pub(in crate::volumes) storage: Box<ChunkStorage>,
    pub(in crate::volumes) pos: P,
}

pub trait PositionStatus: Serialize + DeserializeOwned {
    fn position(&self) -> IVec3 {
        [0, 0, 0].into()
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub struct Positioned(pub IVec3);
impl PositionStatus for Positioned {
    fn position(&self) -> IVec3 {
        self.0
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub struct Unpositioned;
impl PositionStatus for Unpositioned {}

impl<P: PositionStatus> Chunk<P> {
    pub const SIZE: i64 = SIZE;
    pub const USIZE: usize = Self::SIZE as usize;
}

#[allow(clippy::new_without_default)] // please shut up clippy
impl Chunk<Unpositioned> {
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: Box::new(ChunkStorage(
                [[[None; Self::USIZE]; Self::USIZE]; Self::USIZE],
            )),
            pos: Unpositioned,
        }
    }

    #[inline]
    pub fn to_positioned(self, position: IVec3) -> Chunk<Positioned> {
        Chunk {
            storage: self.storage,
            pos: Positioned(position),
        }
    }
}

impl Chunk<Positioned> {
    #[inline]
    pub fn new(pos: IVec3) -> Self {
        Self {
            storage: Box::new(ChunkStorage(
                [[[None; Self::USIZE]; Self::USIZE]; Self::USIZE],
            )),
            pos: Positioned(pos),
        }
    }

    #[inline]
    pub fn to_unpositioned(self) -> Chunk<Unpositioned> {
        Chunk {
            storage: self.storage,
            pos: Unpositioned,
        }
    }
}

impl std::fmt::Debug for Chunk<Unpositioned> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>()).finish()
    }
}

impl std::fmt::Debug for Chunk<Positioned> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bounds = self.pos.0..(self.pos.0 + na::vector![Self::SIZE, Self::SIZE, Self::SIZE]);
        f.debug_struct(type_name::<Self>())
            .field("bounds", &bounds)
            .finish()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoxelSlot {
    OutOfBounds,
    Empty,
    Occupied(BlockId),
}

impl From<VoxelSlot> for Option<BlockId> {
    fn from(slot: VoxelSlot) -> Self {
        match slot {
            VoxelSlot::Occupied(voxel) => Some(voxel),
            _ => None,
        }
    }
}

impl<P: PositionStatus> Volume for Chunk<P> {
    type Input = BlockId;
    type Output = VoxelSlot;

    #[inline]
    fn set(&mut self, idx: Idx, item: Self::Input) -> bool {
        let idx = idx - self.pos.position() * Self::SIZE;

        self.storage.0.set(idx, Some(item))
    }

    #[inline]
    fn get(&self, idx: Idx) -> Self::Output {
        let idx = idx - self.pos.position() * Self::SIZE;

        match self.storage.0.get(idx) {
            Some(block) => match block {
                Some(voxel) => VoxelSlot::Occupied(voxel),
                None => VoxelSlot::Empty,
            },
            None => VoxelSlot::OutOfBounds,
        }
    }

    #[inline]
    fn bounding_box(&self) -> BoundingBox {
        let pos = self.pos.position() * Self::SIZE;
        (pos..(na::vector![Self::SIZE, Self::SIZE, Self::SIZE] + pos)).into()
    }
}
