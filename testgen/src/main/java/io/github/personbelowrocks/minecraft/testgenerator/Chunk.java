package io.github.personbelowrocks.minecraft.testgenerator;

import com.sk89q.worldedit.math.BlockVector3;
import org.jetbrains.annotations.Nullable;

public class Chunk {
    private final BlockVector3 pos;

    private final BlockVector3 min;
    private final BlockVector3 max;

    private final long[][][] storage;

    public Chunk(@Nullable BlockVector3 pos, long[][][] storage) {
        this.pos = pos;

        if (this.pos == null) {
            this.min = BlockVector3.at(0, 0 ,0);
        } else {
            this.min = pos;
        }

        this.max = this.min.add(BlockVector3.at(16, 16, 16));

        this.storage = storage;
    }

    public BlockVector3 getPos() {
        return pos;
    }

    public long[][][] getStorage() {
        return storage;
    }

    public BlockVector3 getMax() {
        return max;
    }

    public BlockVector3 getMin() {
        return min;
    }
}
