use std::io::Write;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use jni::objects::JClass;
use jni::sys::{jbyteArray, jlong, jobject, jshort};
use jni::JNIEnv;
use procgen_common::packets::{self, GenerateRegion, Packet, PacketBuffer};

use crate::{FromJvm, ToJvm};

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_decodePacket(
    env: JNIEnv,
    _class: JClass,
    bytes: jbyteArray,
    // TODO: support size hinting
    _size_hint: jlong,
) -> jobject {
    let bytes = env.convert_byte_array(bytes).unwrap();

    let buffer = {
        let mut reader = ZlibDecoder::new(bytes.as_slice());
        PacketBuffer::from_reader(&mut reader).unwrap()
    };

    let obj = match buffer.id() {
        packets::VoxelData::ID => packets::VoxelData::from_bincode(&buffer)
            .unwrap()
            .to_jvm(env)
            .unwrap(),
        packets::FinishRequest::ID => packets::FinishRequest::from_bincode(&buffer)
            .unwrap()
            .to_jvm(env)
            .unwrap(),
        _ => panic!("Invalid ID"),
    };

    obj.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_encodePacket(
    env: JNIEnv,
    _class: JClass,
    id: jshort,
    jpacket: jobject,
) -> jbyteArray {
    let packet_buffer = match id as u16 {
        packets::GenerateRegion::ID => GenerateRegion::from_jvm(env, jpacket.into())
            .unwrap()
            .to_bincode()
            .unwrap(),
        _ => panic!("Invalid ID"),
    };

    let decompressed_size = packet_buffer.len() as u32;

    let compressed_buffer = {
        let mut buf = vec![];
        let mut writer = ZlibEncoder::new(&mut buf, Compression::best());
        writer.write_all(packet_buffer.as_ref()).unwrap();
        writer.finish().unwrap();
        buf
    };

    let compressed_size = compressed_buffer.len() as u32;

    let mut out_buffer = vec![];
    out_buffer.extend_from_slice(&compressed_size.to_be_bytes());
    out_buffer.extend_from_slice(&decompressed_size.to_be_bytes());

    out_buffer.extend_from_slice(&compressed_buffer);

    env.byte_array_from_slice(&out_buffer).unwrap()
}
