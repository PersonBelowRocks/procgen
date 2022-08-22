package io.github.personbelowrocks.minecraft.testgenerator

import com.sk89q.worldedit.math.BlockVector3

class Chunk(
    val pos: BlockVector3?,
    val storage: Array<Array<Array<Long>>>,
) {
    val min = pos ?: BlockVector3.at(0, 0, 0)
    val max = (pos ?: BlockVector3.at(0, 0, 0)).add(BlockVector3.at(16, 16, 16))
}