use crate::{CtorArgs, JvmConstructable, NamedJObject, QualifiedJValue};
use common::packets::*;
use jni::objects::JValue;
use jni::JNIEnv;
use procgen_common::Chunk;

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

    fn from_jvm_obj(_env: &JNIEnv<'_>, _obj: jni::objects::JObject<'_>) -> Option<Self> {
        // we're probably never gonna need an implementation here because we don't need to transmit this packet to the server.
        todo!()
    }
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

impl JvmConstructable for ConfirmGeneratorAddition {
    const CLASS: &'static str =
        "io/github/personbelowrocks/minecraft/testgenerator/ConfirmGeneratorAddition";

    fn ctor_args<'a>(&self, _env: &JNIEnv<'a>) -> CtorArgs<'a> {
        let mut args = CtorArgs::new();

        args.add(QualifiedJValue::Long(self.request_id.0.into()))
            .add(QualifiedJValue::Long(self.generator_id.0.into()));

        args
    }

    fn from_jvm_obj(_env: &JNIEnv<'_>, _obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
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

    fn from_jvm_obj(_env: &JNIEnv<'_>, _obj: jni::objects::JObject<'_>) -> Option<Self> {
        todo!()
    }
}
