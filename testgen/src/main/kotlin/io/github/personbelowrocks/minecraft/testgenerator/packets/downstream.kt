package io.github.personbelowrocks.minecraft.testgenerator.packets

import io.github.personbelowrocks.minecraft.testgenerator.Chunk

class VoxelData(
    override val requestId: Long,
    val chunk: Chunk
): DownstreamResponse(4, requestId)

class FinishRequest(
    override val requestId: Long
): DownstreamResponse(3, requestId)

class AckRequest(
    override val requestId: Long,
    val info: String?
): DownstreamResponse(5, requestId)

class ListGenerators(
    override val requestId: Long,
    val generators: Array<String>,
): DownstreamResponse(7, requestId)