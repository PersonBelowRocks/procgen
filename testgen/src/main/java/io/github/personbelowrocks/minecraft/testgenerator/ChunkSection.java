package io.github.personbelowrocks.minecraft.testgenerator;

public class ChunkSection {
    private int[][][] internal;

    public ChunkSection(int[][][] arr) {
        internal = arr;
    }

    public int[][][] getInternal() {
        return internal;
    }
}
