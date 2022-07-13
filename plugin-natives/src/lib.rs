extern crate nalgebra as na;

use flate2::read::ZlibDecoder;
use flate2::read::ZlibEncoder;
use flate2::Compression;
use jni::descriptors::Desc;
use jni::objects::{JObject, JValue};
use jni::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
use jni::JNIEnv;
use std::io::Read;

pub mod bindings;
mod chunk;
mod packets;

macro_rules! impl_display_debug {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

macro_rules! impl_from_u32_id {
    ($t:ty) => {
        impl From<u32> for $t {
            fn from(n: u32) -> Self {
                Self(n)
            }
        }

        impl From<$t> for u32 {
            fn from(id: $t) -> Self {
                id.0
            }
        }
    };
}

fn decompress_packet(bytes: &[u8], size_hint: usize) -> Vec<u8> {
    let mut reader = ZlibDecoder::new(bytes);
    let mut buf = Vec::with_capacity(size_hint);

    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn compress_packet(bytes: &[u8]) -> Vec<u8> {
    let mut reader = ZlibEncoder::new(bytes, Compression::best());
    let mut buf = Vec::new();

    reader.read_to_end(&mut buf).unwrap();
    buf
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RequestId(u32);

impl_display_debug!(RequestId);
impl_from_u32_id!(RequestId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GeneratorId(u32);

impl_display_debug!(GeneratorId);
impl_from_u32_id!(GeneratorId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockId(u32);

impl BlockId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl_display_debug!(BlockId);
impl_from_u32_id!(BlockId);

struct JvmConstructableDesc<'a> {
    class: &'static str,
    ctor_sig: String,
    ctor_args: CtorArgs<'a>,
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
        if let QualifiedJValue::Object(obj) = self {
            obj.sig.clone()
        } else {
            match self {
                QualifiedJValue::Byte(_) => "B",
                QualifiedJValue::Char(_) => "C",
                QualifiedJValue::Double(_) => "D",
                QualifiedJValue::Float(_) => "F",
                QualifiedJValue::Int(_) => "I",
                QualifiedJValue::Long(_) => "J",
                QualifiedJValue::Short(_) => "S",
                QualifiedJValue::Void => "V",
                QualifiedJValue::Bool(_) => "Z",
                _ => unreachable!(),
            }
            .into()
        }
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

impl<'b, 'a> CtorArgs<'a> {
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
}

trait JvmConstructable: Sized {
    const CLASS: &'static str;

    fn ctor_args<'b, 'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a>;
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self>;

    fn desc<'b, 'a>(&self, env: &JNIEnv<'a>) -> JvmConstructableDesc<'a> {
        let ctor_args = self.ctor_args(env);

        JvmConstructableDesc {
            class: Self::CLASS,
            ctor_sig: ctor_args.signature(),
            ctor_args,
        }
    }
}
