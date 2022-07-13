package io.github.personbelowrocks.minecraft.testgenerator

interface Packet {
    fun toBytes(): Array<Byte>;
}

class Chunk(val sections: Array<ChunkSection>, val x1: Long, val y1: Long, val z1: Long, val x2: Long, val y2: Long, val z2: Long) {
    fun len(): Int {
        return this.sections.size
    }

    override fun toString(): String {
        return "Chunk {" +
                "   " +
                "}"
    }
}

class ReplyChunk(
    val requestId: Long,
    val chunk: Chunk,
)

class GenerateChunk(
    val requestId: Long,
    val generatorId: Long,
    val x: Int,
    val y: Int
): Packet {
    override fun toString(): String {
        return "GenerateChunk {\n" +
                "   requestId: ${this.requestId},\n" +
                "   generatorId: ${this.generatorId},\n" +
                "   x: ${this.x},\n" +
                "   y: ${this.y},\n" +
                "}"
    }

    override fun toBytes(): Array<Byte> {
        return NativeBindings.encodePacket(0, this).toTypedArray()
    }
}

class AddGenerator(
    val requestId: Long,
    val name: String,
    val minHeight: Int,
    val maxHeight: Int,
    val defaultId: Long,
): Packet {
    override fun toBytes(): Array<Byte> {
        return NativeBindings.encodePacket(2, this).toTypedArray()
    }
}

class ConfirmGeneratorAddition(
    val requestId: Long,
    val generatorId: Long,
)

class ProtocolError(
    val fatal: Boolean,
    val errorMessage: String,
)