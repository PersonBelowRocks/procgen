use jni::objects::JClass;
use jni::sys::{jbyteArray, jlong, jobject, jshort};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_decodePacket(
    env: JNIEnv,
    _class: JClass,
    bytes: jbyteArray,
    size_hint: jlong,
) -> jobject {
    todo!()
}

#[no_mangle]
pub extern "system" fn Java_io_github_personbelowrocks_minecraft_testgenerator_NativeBindings_encodePacket(
    env: JNIEnv,
    _class: JClass,
    id: jshort,
    jpacket: jobject,
) -> jbyteArray {
    todo!()
}
