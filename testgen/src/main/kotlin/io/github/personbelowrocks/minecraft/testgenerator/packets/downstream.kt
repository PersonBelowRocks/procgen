package io.github.personbelowrocks.minecraft.testgenerator.packets

import io.github.personbelowrocks.minecraft.testgenerator.Chunk

class VoxelData(
    val requestId: Long,
    val chunk: Chunk
)

class FinishRequest(
    val requestId: Long
)

class AckRequest(
    val requestId: Long,
    val info: String?
)

class ListGenerators(
    val requestId: Long,
    val generators: Array<String>,
)