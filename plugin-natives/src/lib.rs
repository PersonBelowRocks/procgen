extern crate nalgebra as na;
extern crate procgen_common as common;
extern crate thiserror as te;

use common::packets::PacketBuffer;
use common::BlockId;
use common::Chunk;
use common::PositionStatus;
use common::Positioned;
use common::Unpositioned;
use common::VoxelSlot;
use flate2::read::ZlibDecoder;
use flate2::read::ZlibEncoder;
use flate2::Compression;
use jni::descriptors::Desc;
use jni::objects::JClass;
use jni::objects::{JObject, JValue};
use jni::sys::_jobject;
use jni::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
use jni::JNIEnv;
use std::any::type_name;
use std::io::Read;
use volume::Volume;

pub mod bindings;
mod packets;

const J_NULL: *mut _jobject = std::ptr::null_mut::<_jobject>();

fn decompress_packet(bytes: &[u8], _size_hint: usize) -> PacketBuffer {
    let mut reader = ZlibDecoder::new(bytes);
    PacketBuffer::from_reader(&mut reader).unwrap()
}

fn compress_packet(bytes: &PacketBuffer) -> Vec<u8> {
    let mut reader = ZlibEncoder::new(bytes.as_ref(), Compression::best());
    let mut buf = Vec::new();

    reader.read_to_end(&mut buf).unwrap();
    buf
}

#[derive(Clone)]
struct NamedJObject<'a> {
    sig: String,
    obj: JObject<'a>,
}

impl<'a> NamedJObject<'a> {
    fn new(sig: String, obj: JObject<'a>) -> Self {
        Self { sig, obj }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
enum QualifiedJValue<'a> {
    Object(NamedJObject<'a>),
    Byte(jbyte),
    Char(jchar),
    Short(jshort),
    Int(jint),
    Long(jlong),
    Bool(jboolean),
    Float(jfloat),
    Double(jdouble),
    Void,
}

impl<'a> QualifiedJValue<'a> {
    fn type_signature(&self) -> String {
        match self {
            QualifiedJValue::Byte(_) => "B",
            QualifiedJValue::Char(_) => "C",
            QualifiedJValue::Double(_) => "D",
            QualifiedJValue::Float(_) => "F",
            QualifiedJValue::Int(_) => "I",
            QualifiedJValue::Long(_) => "J",
            QualifiedJValue::Short(_) => "S",
            QualifiedJValue::Bool(_) => "Z",
            QualifiedJValue::Object(obj) => &obj.sig,
            QualifiedJValue::Void => "V",
        }
        .into()
    }
}

impl<'a> From<NamedJObject<'a>> for QualifiedJValue<'a> {
    fn from(obj: NamedJObject<'a>) -> Self {
        Self::Object(obj)
    }
}

impl<'a> From<QualifiedJValue<'a>> for JValue<'a> {
    fn from(val: QualifiedJValue<'a>) -> Self {
        match val {
            QualifiedJValue::Object(obj) => JValue::Object(obj.obj),
            QualifiedJValue::Byte(a) => JValue::Byte(a),
            QualifiedJValue::Char(a) => JValue::Char(a),
            QualifiedJValue::Short(a) => JValue::Short(a),
            QualifiedJValue::Int(a) => JValue::Int(a),
            QualifiedJValue::Long(a) => JValue::Long(a),
            QualifiedJValue::Bool(a) => JValue::Bool(a),
            QualifiedJValue::Float(a) => JValue::Float(a),
            QualifiedJValue::Double(a) => JValue::Double(a),
            QualifiedJValue::Void => JValue::Void,
        }
    }
}

struct CtorArgs<'a> {
    buf: Vec<QualifiedJValue<'a>>,
}

#[allow(dead_code)]
impl<'a> CtorArgs<'a> {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn add(&mut self, val: QualifiedJValue<'a>) -> &mut Self {
        self.buf.push(val);
        self
    }

    fn len(&self) -> usize {
        self.buf.len()
    }

    fn jvalue_buf(&self) -> Vec<JValue<'a>> {
        self.buf
            .iter()
            .cloned()
            .map(|val| val.into())
            .collect::<Vec<_>>()
    }

    fn signature(&self) -> String {
        let args = self
            .buf
            .iter()
            .map(|arg| arg.type_signature())
            .collect::<String>();
        format!("({})V", args)
    }

    fn construct<'c, C: Desc<'a, JClass<'c>>>(
        &self,
        class: C,
        env: &JNIEnv<'a>,
    ) -> Result<JObject<'a>, jni::errors::Error> {
        env.new_object(class, self.signature(), &self.jvalue_buf())
    }
}

pub(crate) trait ToJvm<'a> {
    fn to_jvm(&self, env: JNIEnv<'a>) -> ConvResult<JObject<'a>>;
}

pub(crate) trait FromJvm<'a>: Sized {
    fn from_jvm(env: JNIEnv<'a>, obj: JObject<'a>) -> ConvResult<Self>;
}

#[derive(Copy, Clone)]
pub(crate) struct EnvObject<'a> {
    obj: JObject<'a>,
    env: JNIEnv<'a>,
}

impl<'a> EnvObject<'a> {
    pub fn new(env: JNIEnv<'a>, obj: JObject<'a>) -> Self {
        Self { obj, env }
    }

    pub fn jcall<T>(&self, name: &str, sig: &str, args: &[JValue<'a>]) -> ConvResult<T>
    where
        T: TryFrom<JValue<'a>, Error = jni::errors::Error>,
    {
        let result: Result<T, jni::errors::Error> =
            self.env.call_method(self.obj, name, sig, args)?.try_into();

        match result {
            Ok(out) => Ok(out),
            Err(error) => {
                if matches!(error, jni::errors::Error::WrongJValueType(_, _)) {
                    Err(JvmConversionError::InvalidObject(type_name::<Self>()))
                } else {
                    panic!("Unexpected JNI error: {}", error)
                }
            }
        }
    }
}

pub(crate) struct BlockVector3<'a> {
    obj: EnvObject<'a>,
}

impl<'a> BlockVector3<'a> {
    pub fn get_x(&self) -> i32 {
        self.obj.jcall::<i32>("getX", "()I", &[]).unwrap()
    }

    pub fn get_y(&self) -> i32 {
        self.obj.jcall::<i32>("getY", "()I", &[]).unwrap()
    }

    pub fn get_z(&self) -> i32 {
        self.obj.jcall::<i32>("getZ", "()I", &[]).unwrap()
    }
}

impl<'a, N: From<i32>> From<BlockVector3<'a>> for na::Vector3<N> {
    fn from(bvec: BlockVector3<'a>) -> Self {
        Self::new(
            bvec.get_x().into(),
            bvec.get_y().into(),
            bvec.get_z().into(),
        )
    }
}

pub static WORLDEDIT_BLOCK_VECTOR_3_PATH: &str = "com/sk89q/worldedit/math/BlockVector3";
impl<'a> FromJvm<'a> for BlockVector3<'a> {
    fn from_jvm(env: JNIEnv<'a>, obj: JObject<'a>) -> ConvResult<Self> {
        let expected_class = env.find_class(WORLDEDIT_BLOCK_VECTOR_3_PATH)?;
        let provided_class = env.get_object_class(obj)?;

        if !env.is_same_object(expected_class, provided_class)? {
            Err(JvmConversionError::InvalidObject(
                std::any::type_name::<Self>(),
            ))
        } else {
            Ok(Self {
                obj: EnvObject::new(env, obj),
            })
        }
    }
}

impl<'a> ToJvm<'a> for BlockVector3<'a> {
    fn to_jvm(&self, _env: JNIEnv<'a>) -> ConvResult<JObject<'a>> {
        Ok(self.obj.obj)
    }
}

impl<'a> ToJvm<'a> for na::Vector3<i64> {
    fn to_jvm(&self, env: JNIEnv<'a>) -> ConvResult<JObject<'a>> {
        let mut args = CtorArgs::new();
        args.add(QualifiedJValue::Int(self.x as i32))
            .add(QualifiedJValue::Int(self.y as i32))
            .add(QualifiedJValue::Int(self.z as i32));

        Ok(args.construct(env.find_class(WORLDEDIT_BLOCK_VECTOR_3_PATH)?, &env)?)
    }
}

fn chunk_storage_as_jarray<'a, P: PositionStatus>(
    env: JNIEnv<'a>,
    chunk: &Chunk<P>,
) -> Result<JObject<'a>, jni::errors::Error> {
    let outer = env.new_object_array(Chunk::<P>::SIZE as i32, env.find_class("[[[J")?, J_NULL)?;

    for x in 0..Chunk::<P>::SIZE {
        let middle =
            env.new_object_array(Chunk::<P>::SIZE as i32, env.find_class("[[J")?, J_NULL)?;
        for y in 0..Chunk::<P>::SIZE {
            let inner = env.new_long_array(Chunk::<P>::SIZE as i32)?;
            let mut buf = [VoxelSlot::Empty; 16];

            for z in 0..Chunk::<P>::SIZE {
                let ls_pos = na::vector![x, y, z];
                let pos = chunk.bounding_box().min() + ls_pos;
                buf[z as usize] = chunk.get(pos);
            }

            let buf = buf
                .into_iter()
                .map(<VoxelSlot as Into<Option<BlockId>>>::into)
                .map(|slot| match slot {
                    Some(id) => id.0 as i64,
                    None => i64::MAX,
                })
                .collect::<Vec<_>>();

            env.set_long_array_region(inner, 0, &buf)?;
            env.set_object_array_element(middle, y as i32, inner)?;
        }

        env.set_object_array_element(outer, x as i32, middle)?;
    }

    Ok(outer.into())
}

pub static CHUNK_PATH: &str = "io/github/personbelowrocks/minecraft/testgenerator/Chunk";

impl<'a> ToJvm<'a> for Chunk<Positioned> {
    fn to_jvm(&self, env: JNIEnv<'a>) -> ConvResult<JObject<'a>> {
        let storage = chunk_storage_as_jarray(env, self)?;

        let mut args = CtorArgs::new();
        args.add(QualifiedJValue::Object(NamedJObject::new(
            "[[[J".into(),
            storage,
        )));
        args.add(QualifiedJValue::Object(NamedJObject::new(
            format!("L{};", WORLDEDIT_BLOCK_VECTOR_3_PATH),
            self.bounding_box().min().to_jvm(env)?,
        )));

        Ok(args.construct(env.find_class(CHUNK_PATH)?, &env)?)
    }
}

impl<'a> ToJvm<'a> for Chunk<Unpositioned> {
    fn to_jvm(&self, env: JNIEnv<'a>) -> ConvResult<JObject<'a>> {
        let storage = chunk_storage_as_jarray(env, self)?;

        let mut args = CtorArgs::new();
        args.add(QualifiedJValue::Object(NamedJObject::new(
            "[[[J".into(),
            storage,
        )));
        args.add(QualifiedJValue::Object(NamedJObject::new(
            format!("L{};", WORLDEDIT_BLOCK_VECTOR_3_PATH),
            J_NULL.into(),
        )));

        Ok(args.construct(env.find_class(CHUNK_PATH)?, &env)?)
    }
}

#[derive(te::Error, Debug)]
pub(crate) enum JvmConversionError {
    #[error("Java error: {0}")]
    JavaError(#[from] jni::errors::Error),
    #[error("Cannot convert the given object to type {0}")]
    InvalidObject(&'static str),
}

type ConvResult<T> = std::result::Result<T, JvmConversionError>;
