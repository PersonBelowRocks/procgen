package io.github.personbelowrocks.minecraft.testgenerator.packets

import io.github.personbelowrocks.minecraft.testgenerator.Chunk

class VoxelData(
    override val requestId: Int,
    val chunk: Chunk
): DownstreamResponse(4, requestId)

class FinishRequest(
    override val requestId: Int
): DownstreamResponse(3, requestId)

class AckRequest(
    override val requestId: Int,
    val info: String?
): DownstreamResponse(5, requestId)

class ListGenerators(
    override val requestId: Int,
    val generators: Array<String>,
): DownstreamResponse(7, requestId)