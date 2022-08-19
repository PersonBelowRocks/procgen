static PATH: &str = "io/github/personbelowrocks/minecraft/testgenerator/packets";

pub(self) fn pkt_cls_path(name: &str) -> String {
    let mut path = PATH.to_string();
    path.push('/');
    path.push_str(name);
    path
}

pub(crate) mod upstream {
    use common::{Parameters, RequestId};
    use jni::objects::{JObject, JString};
    use procgen_common::packets;

    use crate::{BlockVector3, EnvObject, FromJvm, WORLDEDIT_BLOCK_VECTOR_3_PATH};

    impl<'a> FromJvm<'a> for packets::GenerateRegion {
        fn from_jvm(
            env: jni::JNIEnv<'a>,
            raw_obj: jni::objects::JObject<'a>,
        ) -> crate::ConvResult<Self> {
            let bvec_getter_sig = format!("L{};", WORLDEDIT_BLOCK_VECTOR_3_PATH);

            let obj = EnvObject::new(env, raw_obj);

            let request_id: RequestId =
                (obj.jcall::<i64>("getRequestId", "()J", &[])? as u32).into();

            let pos1: na::Vector3<i64> = BlockVector3::from_jvm(
                env,
                obj.jcall::<JObject<'a>>("getPos1", &bvec_getter_sig, &[])?,
            )?
            .into();

            let pos2: na::Vector3<i64> = BlockVector3::from_jvm(
                env,
                obj.jcall::<JObject<'a>>("getPos2", &bvec_getter_sig, &[])?,
            )?
            .into();

            let name: String = env
                .get_string(JString::from(obj.jcall::<JObject<'a>>(
                    "getName",
                    "()Ljava/lang/String;",
                    &[],
                )?))?
                .into();

            Ok(Self {
                request_id,
                bounds: pos1..pos2,
                params: Parameters {
                    generator_name: name,
                },
            })
        }
    }

    impl<'a> FromJvm<'a> for packets::GenerateBrush {
        fn from_jvm(
            env: jni::JNIEnv<'a>,
            raw_obj: jni::objects::JObject<'a>,
        ) -> crate::ConvResult<Self> {
            let bvec_getter_sig = format!("L{};", WORLDEDIT_BLOCK_VECTOR_3_PATH);

            let obj = EnvObject::new(env, raw_obj);

            let request_id: RequestId =
                (obj.jcall::<i64>("getRequestId", "()J", &[])? as u32).into();

            let pos: na::Vector3<i64> = BlockVector3::from_jvm(
                env,
                obj.jcall::<JObject<'a>>("getPos", &bvec_getter_sig, &[])?,
            )?
            .into();

            let name: String = env
                .get_string(JString::from(obj.jcall::<JObject<'a>>(
                    "getName",
                    "()Ljava/lang/String;",
                    &[],
                )?))?
                .into();

            Ok(Self {
                request_id,
                pos,
                params: Parameters {
                    generator_name: name,
                },
            })
        }
    }

    impl<'a> FromJvm<'a> for packets::RequestGenerators {
        fn from_jvm(
            env: jni::JNIEnv<'a>,
            raw_obj: jni::objects::JObject<'a>,
        ) -> crate::ConvResult<Self> {
            let obj = EnvObject::new(env, raw_obj);

            let request_id: RequestId =
                (obj.jcall::<i64>("getRequestId", "()J", &[])? as u32).into();

            Ok(Self { request_id })
        }
    }
}

pub(crate) mod downstream {
    use jni::objects::JObject;
    use procgen_common::packets;

    use crate::{CtorArgs, NamedJObject, QualifiedJValue, ToJvm, CHUNK_PATH, J_NULL};

    use super::pkt_cls_path;

    impl<'a> ToJvm<'a> for packets::VoxelData {
        fn to_jvm(&self, env: jni::JNIEnv<'a>) -> crate::ConvResult<jni::objects::JObject<'a>> {
            let mut args = CtorArgs::new();

            args.add(QualifiedJValue::Long(self.request_id.0.into()));
            args.add(QualifiedJValue::Object(NamedJObject::new(
                format!("L{};", CHUNK_PATH),
                self.data.to_jvm(env)?,
            )));

            Ok(args.construct(env.find_class(pkt_cls_path("VoxelData"))?, &env)?)
        }
    }

    impl<'a> ToJvm<'a> for packets::FinishRequest {
        fn to_jvm(&self, env: jni::JNIEnv<'a>) -> crate::ConvResult<jni::objects::JObject<'a>> {
            let mut args = CtorArgs::new();

            args.add(QualifiedJValue::Long(self.request_id.0.into()));
            Ok(args.construct(env.find_class(pkt_cls_path("FinishRequest"))?, &env)?)
        }
    }

    impl<'a> ToJvm<'a> for packets::AckRequest {
        fn to_jvm(&self, env: jni::JNIEnv<'a>) -> crate::ConvResult<jni::objects::JObject<'a>> {
            let mut args = CtorArgs::new();

            args.add(QualifiedJValue::Long(self.request_id.0.into()));

            let info: JObject<'a> = match &self.info {
                Some(string) => env.new_string(string)?.into(),
                None => J_NULL.into(),
            };

            args.add(QualifiedJValue::Object(NamedJObject::new(
                "Ljava/lang/String;".into(),
                info,
            )));

            Ok(args.construct(env.find_class(pkt_cls_path("AckRequest"))?, &env)?)
        }
    }

    impl<'a> ToJvm<'a> for packets::ListGenerators {
        fn to_jvm(&self, env: jni::JNIEnv<'a>) -> crate::ConvResult<jni::objects::JObject<'a>> {
            let mut args = CtorArgs::new();

            args.add(QualifiedJValue::Long(self.request_id.0.into()));

            let generator_list: JObject<'a> = {
                let arr = env.new_object_array(
                    self.generators.len() as i32,
                    env.find_class("java/lang/String")?,
                    J_NULL,
                )?;

                for (i, string) in self.generators.iter().enumerate() {
                    env.set_object_array_element(arr, i as i32, env.new_string(string)?)?;
                }

                arr.into()
            };

            args.add(QualifiedJValue::Object(NamedJObject::new(
                "[Ljava/lang/String;".into(),
                generator_list,
            )));

            Ok(args.construct(env.find_class(pkt_cls_path("ListGenerators"))?, &env)?)
        }
    }
}
