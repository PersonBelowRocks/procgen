package io.github.personbelowrocks.minecraft.testgenerator

import com.sk89q.worldedit.math.BlockVector3

class Chunk(
    val pos: BlockVector3?,
    val storage: Array<Array<Array<Long>>>,
)