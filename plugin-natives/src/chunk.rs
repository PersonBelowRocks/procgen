use crate::{
    BlockId, CtorArgs, JvmConstructable, JvmConstructableDesc, NamedJObject, QualifiedJValue,
};
use jni::descriptors::Desc;
use jni::objects::{JValue, ReleaseMode};
use jni::sys::jobject;
use jni::JNIEnv;
use volume::{BoundingBox, StackVolume, Volume, VolumeAccess, VolumeIdx};

const CHUNK_SIZE: usize = 16;
const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;

type CubicVolume<const N: usize, T> = StackVolume<N, N, N, T>;
type ChunkSectionStorage = CubicVolume<CHUNK_SIZE, BlockId>;

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
            x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE
        } else {
            false
        }
    }
}

impl Volume for ChunkSection {
    type Item = BlockId;
}

pub(super) fn chunk_sections_for_height(height: i32) -> usize {
    debug_assert!(height >= 0); // we can't have chunks with a negative height

    ((height + CHUNK_SIZE_I32 - ((height - 1).rem_euclid(CHUNK_SIZE_I32))) / CHUNK_SIZE_I32)
        as usize
}

#[derive(Clone)]
pub struct Chunk {
    pub(in crate::chunk) sections: Vec<ChunkSection>,
    pub(in crate::chunk) bounding_box: BoundingBox,
}

impl Chunk {
    pub fn new(
        default: BlockId,
        chunk_pos: na::Vector2<i32>,
        min_height: i32,
        max_height: i32,
    ) -> Self {
        let pos = chunk_pos * (CHUNK_SIZE as i32);

        let sections = {
            let capacity = chunk_sections_for_height((min_height - max_height).abs());
            let mut vec = Vec::with_capacity(capacity);

            for _ in 0..capacity {
                vec.push(ChunkSection::new_uninitialized(default));
            }

            debug_assert!(vec.len() == capacity);

            vec
        };

        let bounding_box = BoundingBox::new([pos.x, min_height, pos.y], [pos.x, max_height, pos.y]);

        Self {
            sections,
            bounding_box,
        }
    }

    #[inline]
    fn get_chunk_section(&self, chunk_section_idx: usize) -> Option<&ChunkSection> {
        self.sections.get(chunk_section_idx)
    }

    #[inline]
    fn get_mut_chunk_section(&mut self, chunk_section_idx: usize) -> Option<&mut ChunkSection> {
        self.sections.get_mut(chunk_section_idx)
    }

    #[inline]
    pub fn bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }

    #[inline]
    pub fn default_id(&self) -> BlockId {
        self.sections[0].default_id()
    }
}

impl<Idx: VolumeIdx> VolumeAccess<Idx> for Chunk {
    #[inline]
    fn get(this: &Self, idx: Idx) -> Option<&Self::Item> {
        let [x, y, z] = idx.array::<i32>()?;

        // This is the height/index of the chunk section containing the position provided.
        let section_idx = y / CHUNK_SIZE_I32;
        // This is the position within that section corresponding to the position provided
        // (i.e., in the section's localspace).
        let sectionspace_y = y % CHUNK_SIZE_I32;

        let section = this.get_chunk_section(section_idx as usize)?;
        section.get([x, sectionspace_y, z])
    }

    #[inline]
    fn set(this: &mut Self, idx: Idx, item: Self::Item) {
        if let Some([x, y, z]) = idx.array::<i32>() {
            let section_idx = y / CHUNK_SIZE_I32;
            let sectionspace_y = y % CHUNK_SIZE_I32;

            if let Some(section) = this.get_mut_chunk_section(section_idx as usize) {
                section.set([x, sectionspace_y, z], item);
            }
        }
    }

    #[inline]
    fn swap(this: &mut Self, idx: Idx, item: Self::Item) -> Option<Self::Item> {
        let [x, y, z] = idx.array::<i32>()?;
        let section_idx = y / CHUNK_SIZE_I32;
        let sectionspace_y = y % CHUNK_SIZE_I32;

        let section = this.get_mut_chunk_section(section_idx as usize)?;
        section.swap([x, sectionspace_y, z], item)
    }

    #[inline]
    fn contains(this: &Self, idx: Idx) -> bool {
        if let Some([x, y, z]) = idx.array::<usize>() {
            (y / CHUNK_SIZE as usize) < this.sections.len()
                && x < CHUNK_SIZE as usize
                && z < CHUNK_SIZE as usize
        } else {
            false
        }
    }
}

impl<Idx: VolumeIdx> VolumeAccess<Spaces<Idx>> for Chunk {
    fn get(this: &Self, idx: Spaces<Idx>) -> Option<&Self::Item> {
        let ls_idx = idx.ls(this.bounding_box());
        this.get(ls_idx)
    }

    fn set(this: &mut Self, idx: Spaces<Idx>, item: Self::Item) {
        let ls_idx = idx.ls(this.bounding_box());
        this.set(ls_idx, item);
    }

    fn swap(this: &mut Self, idx: Spaces<Idx>, item: Self::Item) -> Option<Self::Item> {
        let ls_idx = idx.ls(this.bounding_box());
        this.swap(ls_idx, item)
    }

    fn contains(this: &Self, idx: Spaces<Idx>) -> bool {
        match idx {
            Spaces::Ls(idx) => this.contains(idx),
            Spaces::Cs(idx) => {
                if let Some([x, y, z]) = idx.array::<i64>() {
                    this.contains([x, y + this.bounding_box().min()[1], z])
                } else {
                    false
                }
            }
            Spaces::Ws(idx) => {
                // Our bounding box is in worldspace.
                this.bounding_box().contains(idx)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum Spaces<Idx: VolumeIdx> {
    Ls(Idx),
    Cs(Idx),
    Ws(Idx),
}

impl<Idx: VolumeIdx> Spaces<Idx> {
    #[inline]
    fn ls(self, bb: BoundingBox) -> Idx {
        match self {
            Self::Ls(idx) => idx,
            Self::Cs(idx) => {
                let [x, y, z] = idx.array::<i64>().unwrap();
                Idx::from_xyz(x, y - bb.min()[1], z)
            }
            Self::Ws(idx) => {
                let wpos = na::Vector3::from(idx.array::<i64>().unwrap());
                let bb_min = na::Vector3::from(bb.min());

                let lpos = wpos - bb_min;
                let [x, y, z]: [i64; 3] = lpos.into();
                Idx::from_xyz(x, y, z)
            }
        }
    }

    #[inline]
    fn cs(self, _bb: BoundingBox) -> Idx {
        todo!()
    }

    #[inline]
    fn ws(self, _bb: BoundingBox) -> Idx {
        todo!()
    }
}

impl Volume for Chunk {
    type Item = BlockId;
}

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

use serde::de::{Error, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};

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
            Error::custom("sequence was too short and did not contain the default block ID")
        })?;
        let initialized = seq.next_element::<bool>()?.ok_or_else(|| {
            Error::custom(
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
                        .ok_or_else(|| Error::custom("voxel sequence terminated prematurely"))?;

                    section.set([x, y, z], voxel)
                }
            }
        }

        if seq.next_element::<BlockId>()?.is_some() {
            Err(Error::custom("sequence was too long!"))
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
            na::vector![x as i32, z as i32] / CHUNK_SIZE_I32
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
            Error::custom("sequence was too short and did not contain the chunk's position")
        })?;
        let min_height = seq.next_element::<i32>()?.ok_or_else(|| {
            Error::custom("sequence was too short and did not contain the chunk's min height")
        })?;
        let max_height = seq.next_element::<i32>()?.ok_or_else(|| {
            Error::custom("sequence was too short and did not contain the chunk's max height")
        })?;

        let n_sections = chunk_sections_for_height((min_height - max_height).abs());

        let mut sections = Vec::<ChunkSection>::with_capacity(n_sections);
        for _ in 0..n_sections {
            sections.push(seq.next_element::<ChunkSection>()?.ok_or_else(|| {
                Error::custom("sequence was too short and did not contain all sections")
            })?);
        }

        if seq.next_element::<ChunkSection>()?.is_some() {
            Err(Error::custom(format!("sequence was longer than expected given the specified chunk height (min: {}, max: {})", min_height, max_height)))
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

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("bounding_box", &self.bounding_box())
            .field(
                "sections",
                &self
                    .sections
                    .iter()
                    .enumerate()
                    .map(|(i, section)| {
                        let mut s = format!("{i}: (default id {:?}): ", section.default_id());
                        if section.is_initialized() {
                            s.push_str("INITIALIZED");
                        } else {
                            s.push_str("UNINITIALIZED");
                        }
                        s
                    })
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl JvmConstructable for ChunkSection {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/ChunkSection";

    fn ctor_args<'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a> {
        if !self.is_initialized() {
            let mut args = CtorArgs::new();
            args.add(QualifiedJValue::Object(NamedJObject::new(
                "[[[I".into(),
                (std::ptr::null::<u8>() as jobject).into(),
            )));

            return args;
        }

        let cls = env.find_class("[I").unwrap();

        let pole = env.new_int_array(CHUNK_SIZE as _).unwrap();
        let sheet = env.new_object_array(CHUNK_SIZE as _, cls, pole).unwrap();

        let cubic = env
            .new_object_array(CHUNK_SIZE as _, env.get_object_class(sheet).unwrap(), sheet)
            .unwrap();
        for x in 0..CHUNK_SIZE {
            let sheet = env.new_object_array(CHUNK_SIZE as _, cls, pole).unwrap();

            for y in 0..CHUNK_SIZE {
                let pole = env.new_int_array(CHUNK_SIZE as _).unwrap();
                let buf = (0..CHUNK_SIZE)
                    .map(|z| self.volume.as_ref().unwrap()[[x, y, z]])
                    .map(|b| i32::from_be_bytes(u32::from(b).to_be_bytes()))
                    .collect::<Vec<i32>>();

                env.set_int_array_region(pole, 0, &buf).unwrap();
                env.set_object_array_element(sheet, y as _, pole).unwrap();
            }

            env.set_object_array_element(cubic, x as _, sheet).unwrap();
        }

        let mut args = CtorArgs::new();
        args.add(QualifiedJValue::Object(NamedJObject::new(
            "[[[I".into(),
            cubic.into(),
        )));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
    }
}

impl JvmConstructable for Chunk {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/Chunk";

    fn ctor_args<'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let section_cls = env.find_class(ChunkSection::CLASS).unwrap();

        let sections = self
            .sections
            .iter()
            .map(|s| {
                let args = s.ctor_args(env);
                match env.new_object(section_cls, args.signature(), &args.jvalue_buf()) {
                    Ok(obj) => obj,
                    Err(error) => {
                        let jerr = error.lookup(env).unwrap();
                        env.exception_describe().unwrap();
                        panic!("{:?}", jerr)
                    }
                }
            })
            .collect::<Vec<_>>();

        let jvm_sections = env
            .new_object_array(sections.len() as _, section_cls, sections[0])
            .unwrap();
        sections.into_iter().enumerate().for_each(|(i, section)| {
            env.set_object_array_element(jvm_sections, i as _, section)
                .unwrap();
        });

        let mut args = CtorArgs::new();
        args.add(QualifiedJValue::Object(NamedJObject::new(
            format!("[L{};", ChunkSection::CLASS),
            jvm_sections.into(),
        )))
        .add(QualifiedJValue::Long(self.bounding_box.min()[0]))
        .add(QualifiedJValue::Long(self.bounding_box.min()[1]))
        .add(QualifiedJValue::Long(self.bounding_box.min()[2]))
        .add(QualifiedJValue::Long(self.bounding_box.max()[0]))
        .add(QualifiedJValue::Long(self.bounding_box.max()[1]))
        .add(QualifiedJValue::Long(self.bounding_box.max()[2]));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
    }
}
