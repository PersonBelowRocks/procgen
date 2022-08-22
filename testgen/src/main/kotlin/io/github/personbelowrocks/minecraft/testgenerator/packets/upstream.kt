package io.github.personbelowrocks.minecraft.testgenerator.packets

import com.sk89q.worldedit.math.BlockVector3
import io.github.personbelowrocks.minecraft.testgenerator.NativeBindings


class GenerateRegion(
    val requestId: Long,
    val pos1: BlockVector3,
    val pos2: BlockVector3,

    // TODO: currently this just represents a generator's name, but should
    //  represent more extensive parameters for a generator too
    val name: String,
): UpstreamPacket(1)

class GenerateBrush(
    val requestId: Long,
    val pos: BlockVector3,

    // TODO: see the same field on GenerateRegion
    val name: String
): UpstreamPacket(2)

class RequestGenerators(
    val requestId: Long
): UpstreamPacket(6)