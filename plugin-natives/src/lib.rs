extern crate nalgebra as na;
extern crate procgen_common as common;

use common::packets::PacketBuffer;
use common::Chunk;
use common::ChunkSection;
use common::CHUNK_SIZE;
use flate2::read::ZlibDecoder;
use flate2::read::ZlibEncoder;
use flate2::Compression;
use jni::descriptors::Desc;
use jni::objects::JClass;
use jni::objects::{JObject, JValue};
use jni::sys::_jobject;
use jni::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
use jni::JNIEnv;
use std::io::Read;

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

#[allow(dead_code)]
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

    fn call<'c, C: Desc<'a, JClass<'c>>>(&self, class: C, env: &JNIEnv<'a>) -> Option<JObject<'a>> {
        env.new_object(class, self.signature(), &self.jvalue_buf())
            .ok()
    }
}

pub(crate) trait ToJvmObject {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a>;
}

pub(crate) trait FromJvmObject: Sized {
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self>;
}

const CHUNK_SECTION_PATH: &str = "io/github/personbelowrocks/minecraft/testgenerator/ChunkSection";
impl ToJvmObject for ChunkSection {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a> {
        if !self.is_initialized() {
            let mut args = CtorArgs::new();

            args.add(QualifiedJValue::Object(NamedJObject::new(
                "[[[I".into(),
                J_NULL.into(),
            )));

            return args.call(CHUNK_SECTION_PATH, env).unwrap();
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
                    .map(|z| self.inner_ref().unwrap()[[x, y, z]])
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

        args.call(CHUNK_SECTION_PATH, env).unwrap()
    }
}

const CHUNK_PATH: &str = "io/github/personbelowrocks/minecraft/testgenerator/Chunk";
impl ToJvmObject for Chunk {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a> {
        let section_cls = env.find_class(CHUNK_SECTION_PATH).unwrap();

        let sections = self
            .sections()
            .iter()
            .map(|s| s.to_jvm_obj(env))
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
            format!("[L{};", CHUNK_SECTION_PATH),
            jvm_sections.into(),
        )))
        .add(QualifiedJValue::Long(self.bounding_box().min()[0]))
        .add(QualifiedJValue::Long(self.bounding_box().min()[1]))
        .add(QualifiedJValue::Long(self.bounding_box().min()[2]))
        .add(QualifiedJValue::Long(self.bounding_box().max()[0]))
        .add(QualifiedJValue::Long(self.bounding_box().max()[1]))
        .add(QualifiedJValue::Long(self.bounding_box().max()[2]));

        args.call(CHUNK_PATH, env).unwrap()
    }
}

impl FromJvmObject for na::Vector3<i32> {
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self> {
        if let (JValue::Int(x), JValue::Int(y), JValue::Int(z)) = (
            env.call_method(obj, "getX", "()I", &[]).ok()?,
            env.call_method(obj, "getY", "()I", &[]).ok()?,
            env.call_method(obj, "getZ", "()I", &[]).ok()?,
        ) {
            Some(na::vector![x, y, z])
        } else {
            None
        }
    }
}
