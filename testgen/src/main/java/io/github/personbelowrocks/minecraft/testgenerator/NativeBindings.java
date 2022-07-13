package io.github.personbelowrocks.minecraft.testgenerator;

public class NativeBindings {
    private static final String LIB_NAME = "rustgen";

    static {
        try {
            System.loadLibrary(LIB_NAME);
        } catch (UnsatisfiedLinkError e) {
            System.load(NativeGen.getLibPath());
        }
    }

    public static native Object decodePacket(byte[] bytes, long sizeHint);

    public static native byte[] encodePacket(short id, Object packet);

    public static native void takeArray(int[][][] arr);
}
