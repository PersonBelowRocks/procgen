use crate::chunk::Chunk;
use crate::{
    BlockId, CtorArgs, GeneratorId, JvmConstructable, JvmConstructableDesc, NamedJObject,
    QualifiedJValue, RequestId,
};
use jni::objects::JValue;
use jni::JNIEnv;

pub trait Packet: serde::Serialize + serde::de::DeserializeOwned {
    const ID: u16;

    fn to_bincode(&self) -> Vec<u8> {
        let mut buf = Self::ID.to_be_bytes().to_vec();
        buf.extend(bincode::serialize(self).unwrap());
        buf
    }

    fn from_bincode(bytes: &[u8]) -> Option<Self> {
        let id = u16::from_be_bytes(bytes[..2].try_into().ok()?);
        let body = &bytes[2..];

        if id != Self::ID {
            return None;
        }

        bincode::deserialize::<Self>(body).ok()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GenerateChunk {
    pub request_id: RequestId,
    pub generator_id: GeneratorId,
    pub pos: na::Vector2<i32>,
}

impl Packet for GenerateChunk {
    const ID: u16 = 0;
}

impl JvmConstructable for GenerateChunk {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/GenerateChunk";

    fn ctor_args<'a>(&self, _env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()))
            .add(QualifiedJValue::Long(self.generator_id.0.into()))
            .add(QualifiedJValue::Int(self.pos.x))
            .add(QualifiedJValue::Int(self.pos.y));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        match (
            env.call_method(obj, "getRequestId", "()J", &[]).ok()?,
            env.call_method(obj, "getGeneratorId", "()J", &[]).ok()?,
            env.call_method(obj, "getX", "()I", &[]).ok()?,
            env.call_method(obj, "getY", "()I", &[]).ok()?,
        ) {
            (
                JValue::Long(request_id),
                JValue::Long(generator_id),
                JValue::Int(x),
                JValue::Int(y),
            ) => Some(Self {
                request_id: (request_id as u32).into(),
                generator_id: (generator_id as u32).into(),
                pos: na::vector![x, y],
            }),
            _ => None,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplyChunk {
    pub request_id: RequestId,
    pub chunk: Chunk,
}

impl JvmConstructable for ReplyChunk {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/ReplyChunk";

    fn ctor_args<'b, 'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()));

        let chunk_ctor_args = self.chunk.ctor_args(env);
        let chunk_obj = env
            .new_object(
                env.find_class(Chunk::CLASS).unwrap(),
                chunk_ctor_args.signature(),
                &chunk_ctor_args.jvalue_buf(),
            )
            .unwrap();
        args.add(QualifiedJValue::Object(NamedJObject::new(
            format!("L{};", Chunk::CLASS),
            chunk_obj,
        )));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        // we're probably never gonna need an implementation here because we don't need to transmit this packet to the server.
        todo!()
    }
}

impl Packet for ReplyChunk {
    const ID: u16 = 1;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddGenerator {
    pub request_id: RequestId,
    pub name: String,
    pub min_height: i32,
    pub max_height: i32,
    pub default_id: BlockId,
}

impl JvmConstructable for AddGenerator {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/ReplyChunk";

    fn ctor_args<'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()))
            .add(QualifiedJValue::Object(NamedJObject::new(
                "Ljava/lang/String;".into(),
                env.new_string(&self.name).unwrap().into(),
            )))
            .add(QualifiedJValue::Int(self.min_height))
            .add(QualifiedJValue::Int(self.max_height))
            .add(QualifiedJValue::Long(self.default_id.0.into()));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        match (
            env.call_method(obj, "getRequestId", "()J", &[]).ok()?,
            env.call_method(obj, "getName", "()Ljava/lang/String;", &[])
                .ok()?,
            env.call_method(obj, "getMinHeight", "()I", &[]).ok()?,
            env.call_method(obj, "getMaxHeight", "()I", &[]).ok()?,
            env.call_method(obj, "getDefaultId", "()J", &[]).ok()?,
        ) {
            (
                JValue::Long(request_id),
                JValue::Object(jname),
                JValue::Int(min_height),
                JValue::Int(max_height),
                JValue::Long(default_id),
            ) => Some(Self {
                request_id: (request_id as u32).into(),
                name: {
                    let mutf8_jstring = env.get_string(jname.into()).unwrap();
                    mutf8_jstring.into()
                },
                min_height,
                max_height,
                default_id: (default_id as u32).into(),
            }),
            _ => None,
        }
    }
}

impl Packet for AddGenerator {
    const ID: u16 = 2;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConfirmGeneratorAddition {
    pub request_id: RequestId,
    pub generator_id: GeneratorId,
}

impl ConfirmGeneratorAddition {
    pub fn new(request_id: RequestId, generator_id: GeneratorId) -> Self {
        Self {
            request_id,
            generator_id,
        }
    }
}

impl JvmConstructable for ConfirmGeneratorAddition {
    const CLASS: &'static str =
        "io/github/personbelowrocks/minecraft/testgenerator/ConfirmGeneratorAddition";

    fn ctor_args<'a>(&self, _env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()))
            .add(QualifiedJValue::Long(self.generator_id.0.into()));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
    }
}

impl Packet for ConfirmGeneratorAddition {
    const ID: u16 = 3;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ProtocolErrorKind {
    Other {
        details: String,
    },
    GeneratorNotFound {
        generator_id: GeneratorId,
        request_id: RequestId,
    },
    ChunkGenerationFailure {
        generator_id: GeneratorId,
        request_id: RequestId,
        details: String,
    },
    Terminated {
        details: String,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProtocolError {
    pub kind: ProtocolErrorKind,
    pub fatal: bool,
}

impl ProtocolError {
    pub fn gentle(kind: ProtocolErrorKind) -> Self {
        Self { kind, fatal: false }
    }

    pub fn fatal(kind: ProtocolErrorKind) -> Self {
        Self { kind, fatal: true }
    }
}

impl JvmConstructable for ProtocolError {
    const CLASS: &'static str = "io/github/personbelowrocks/minecraft/testgenerator/ProtocolError";

    fn ctor_args<'a>(&self, env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Bool(self.fatal.into()))
            .add(QualifiedJValue::Object(NamedJObject::new(
                "Ljava/lang/String;".into(),
                env.new_string(format!("{:?}", self)).unwrap().into(),
            )));

        args
    }

    fn from_jvm_obj(env: &JNIEnv<'_>, obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
    }
}

impl Packet for ProtocolError {
    const ID: u16 = 4;
}
