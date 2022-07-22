use crate::{CtorArgs, FromJvmObject, NamedJObject, QualifiedJValue, ToJvmObject, CHUNK_PATH};
use common::packets::*;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;

impl FromJvmObject for GenerateChunk {
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self> {
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

const REPLY_CHUNK_PATH: &str = "io/github/personbelowrocks/minecraft/testgenerator/ReplyChunk";
impl ToJvmObject for ReplyChunk {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()));

        let chunk_obj = self.chunk.to_jvm_obj(env);
        args.add(QualifiedJValue::Object(NamedJObject::new(
            format!("L{};", CHUNK_PATH),
            chunk_obj,
        )));

        args.call(REPLY_CHUNK_PATH, env).unwrap()
    }
}

impl FromJvmObject for AddGenerator {
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self> {
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

const CONFIRM_GENERATOR_ADDITION_PATH: &str =
    "io/github/personbelowrocks/minecraft/testgenerator/ConfirmGeneratorAddition";
impl ToJvmObject for ConfirmGeneratorAddition {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()))
            .add(QualifiedJValue::Long(self.generator_id.0.into()));

        args.call(CONFIRM_GENERATOR_ADDITION_PATH, env).unwrap()
    }
}

const PROTOCOL_ERROR_PATH: &str =
    "io/github/personbelowrocks/minecraft/testgenerator/ProtocolError";
impl ToJvmObject for ProtocolError {
    fn to_jvm_obj<'a>(&self, env: &JNIEnv<'a>) -> JObject<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Bool(self.fatal.into()))
            .add(QualifiedJValue::Object(NamedJObject::new(
                "Ljava/lang/String;".into(),
                env.new_string(format!("{:?}", self)).unwrap().into(),
            )));

        args.call(PROTOCOL_ERROR_PATH, env).unwrap()
    }
}

const BLOCK_VECTOR_PATH: &str = "com/sk89q/worldedit/math/BlockVector3";
impl FromJvmObject for GenerateRegion {
    fn from_jvm_obj(env: &JNIEnv<'_>, obj: JObject<'_>) -> Option<Self> {
        if let (
            JValue::Long(request_id),
            JValue::Object(pos1_obj),
            JValue::Object(pos2_obj),
            JValue::Object(_params_obj),
        ) = (
            env.call_method(obj, "getRequestId", "()J", &[]).ok()?,
            env.call_method(obj, "getPos1", &format!("()L{};", BLOCK_VECTOR_PATH), &[])
                .ok()?,
            env.call_method(obj, "getPos2", &format!("()L{};", BLOCK_VECTOR_PATH), &[])
                .ok()?,
            env.call_method(obj, "getParams", "()Ljava/lang/String;", &[])
                .ok()?,
        ) {
            let pos1 = na::Vector3::<i32>::from_jvm_obj(env, pos1_obj)?;
            let pos2 = na::Vector3::<i32>::from_jvm_obj(env, pos2_obj)?;

            let params = Parameters::default();

            Some(GenerateRegion {
                request_id: (request_id as u32).into(),
                bounds: pos1..pos2,
                params,
            })
        } else {
            None
        }
    }
}
