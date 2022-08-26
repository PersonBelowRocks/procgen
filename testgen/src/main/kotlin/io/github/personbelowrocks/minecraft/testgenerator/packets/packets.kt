package io.github.personbelowrocks.minecraft.testgenerator.packets

import io.github.personbelowrocks.minecraft.testgenerator.NativeBindings

abstract class Packet(open val id: Short)

abstract class UpstreamPacket(id: Short): Packet(id) {
    open fun toBytes(): ByteArray {
        return NativeBindings.encodePacket(id, this)
    }
}

abstract class DownstreamPacket(id: Short): Packet(id) {
    companion object {
        fun decode(buf: ByteArray, sizeHint: Long): DownstreamPacket {
            val packet = NativeBindings.decodePacket(buf, sizeHint)
            return packet as DownstreamPacket
        }
    }
}

abstract class DownstreamResponse(id: Short, open val requestId: Int): DownstreamPacket(id)