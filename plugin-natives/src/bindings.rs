use crate::{compress_packet, decompress_packet, jlong, jshort, JvmConstructable};
use common::packets::*;
use jni::objects::JClass;
use jni::sys::{jbyteArray, jobject};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_decodePacket(
    env: JNIEnv,
    _class: JClass,
    bytes: jbyteArray,
    size_hint: jlong,
) -> jobject {
    let compressed_buffer = env.convert_byte_array(bytes).unwrap();
    let decompressed_buffer = decompress_packet(&compressed_buffer, size_hint as _);
    let id = decompressed_buffer.id();

    let desc = match id {
        GenerateChunk::ID => GenerateChunk::from_bincode(&decompressed_buffer)
            .unwrap()
            .desc(&env),
        ReplyChunk::ID => ReplyChunk::from_bincode(&decompressed_buffer)
            .unwrap()
            .desc(&env),
        ConfirmGeneratorAddition::ID => {
            ConfirmGeneratorAddition::from_bincode(&decompressed_buffer)
                .unwrap()
                .desc(&env)
        }
        _ => panic!("invalid packet ID: {id}"),
    };

    let jvm_class = env.find_class(desc.class).unwrap();
    let jvm_packet_obj = env
        .new_object(jvm_class, desc.ctor_sig, &desc.ctor_args.jvalue_buf())
        .unwrap();

    jvm_packet_obj.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_encodePacket(
    env: JNIEnv,
    _class: JClass,
    id: jshort,
    jpacket: jobject,
) -> jbyteArray {
    let decompressed_buffer = match id as u16 {
        AddGenerator::ID => {
            let packet = AddGenerator::from_jvm_obj(&env, jpacket.into()).unwrap();
            packet.to_bincode()
        }
        GenerateChunk::ID => {
            let packet = GenerateChunk::from_jvm_obj(&env, jpacket.into()).unwrap();
            packet.to_bincode()
        }
        _ => panic!("uh oh! {} is not a good ID :(", id),
    }
    .unwrap();

    let decompressed_len = decompressed_buffer.len() as u32;
    let compressed_buffer = compress_packet(&decompressed_buffer);
    let compressed_len = compressed_buffer.len() as u32;

    let mut buf = compressed_len.to_be_bytes().to_vec();
    buf.extend(decompressed_len.to_be_bytes());
    buf.extend(compressed_buffer);

    env.byte_array_from_slice(&buf).unwrap()
}
