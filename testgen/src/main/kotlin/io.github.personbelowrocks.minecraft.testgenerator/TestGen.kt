package io.github.personbelowrocks.minecraft.testgenerator;

import org.bukkit.Bukkit
import org.bukkit.Material
import org.bukkit.World
import org.bukkit.block.data.BlockData
import org.bukkit.event.EventHandler
import org.bukkit.event.Listener
import org.bukkit.event.world.ChunkLoadEvent
import org.bukkit.generator.ChunkGenerator
import org.bukkit.generator.WorldInfo
import org.bukkit.plugin.java.JavaPlugin
import java.io.*
import java.net.Socket
import java.util.*
import java.util.concurrent.LinkedBlockingQueue
import kotlin.collections.HashMap
import kotlin.math.E
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.pow


val logger = Bukkit.getLogger()


class ChunkEvents(private val cache: HashMap<ChunkPosition, CachedChunk>, private val batchSize: Int, private val nLatest: Int, private val cleanupInterval: Int): Listener {
    private val latest = HashSet<ChunkPosition>()
    private var c: Int = 0

    @EventHandler
    fun onChunkGenerate(event: ChunkLoadEvent) {
        if (!event.isNewChunk) {
            return
        }

        val pos = ChunkPosition(
            event.chunk.x,
            event.chunk.z,
            event.world.uid
        )

        if (latest.find { it.worldId == pos.worldId && abs(it.x - pos.x) <= batchSize && abs(it.z - pos.z) <= batchSize } == null) {
            return
        }

        if (latest.size + 1 > nLatest) {
            latest.remove(latest.maxByOrNull { (it.x * it.x + it.z * it.z) })
        }

        latest.add(pos)

        c += 1
        if (c >= cleanupInterval) {
            c = 0

            cache.filterKeys {cachePos ->
                latest.find {latestPos -> cachePos.worldId == latestPos.worldId && abs(cachePos.x - latestPos.x) <= batchSize && abs(cachePos.z - latestPos.z) <= batchSize } == null
            }.forEach { (pos, _) -> cache.remove(pos) }
        }
    }
}


class Main : JavaPlugin() {
    private val generator = MyGenerator(this, "127.0.0.1", 44332u)

    override fun getDefaultWorldGenerator(worldName: String, id: String?): ChunkGenerator? {
        return generator
    }

    override fun onEnable() {
        generator.start()

        logger.info("hello")
    }

    override fun onDisable() {
        println("goodbye")
    }
}

data class ChunkPosition(
    val x: Int,
    val z: Int,
    val worldId: UUID,
)

data class CachedChunk(
    val world: World,
    val chunk: Chunk?
)

class ChunkProvider(inStream: InputStream, outStream: OutputStream, private val batchSize: Int) {
    private val networker = Networker(inStream, outStream)
    private val cache = HashMap<ChunkPosition, CachedChunk>()

    fun getChunk(generatorId: Long, pos: ChunkPosition, world: World): Chunk {
        if (cache.containsKey(pos)) {
            val chunk = cache[pos]!!.chunk
            if (chunk != null) {
                return chunk
            }
        }

        val packets = ArrayList<GenerateChunk>()

        for (x in -batchSize..batchSize) {
            for (z in -batchSize..batchSize) {
                val candidatePos = ChunkPosition(
                    x,
                    z,
                    world.uid
                )

                if (cache.containsKey(candidatePos) || world.isChunkGenerated(x, z)) {
                    continue
                } else {
                    TODO()
                }
            }
        }

        TODO()
    }
}

class Networker(inStream: InputStream, outStream: OutputStream) {
    private val inputStream = inStream.buffered()
    private val outputStream = outStream.buffered()

    private val outboundPacketQueue = LinkedBlockingQueue<GenerateChunk>()

    private val activeRequests = HashMap<Long, LinkedBlockingQueue<Chunk>>()

    class NetWriter(private val stream: BufferedOutputStream, private val queue: LinkedBlockingQueue<GenerateChunk>): Runnable {
        override fun run() {
            while (true) {
                val packet = queue.take()

                val buf = packet.toBytes().toByteArray()
                stream.write(buf)
                stream.flush()
            }
        }
    }

    class NetReader(private val stream: BufferedInputStream, private val map: Map<Long, LinkedBlockingQueue<Chunk>>): Runnable {
        override fun run() {
            while (true) {

                val compressedSize = stream.readNBytes(4).getUIntAt(0)
                val decompressedSize = stream.readNBytes(4).getUIntAt(0)
                val compressedBuffer = stream.readNBytes(compressedSize.toInt())

                val packet = NativeBindings.decodePacket(compressedBuffer, decompressedSize.toLong()) as ReplyChunk

                map[packet.requestId]!!.put(packet.chunk)
            }
        }
    }

    fun start() {
        Thread(NetWriter(outputStream, outboundPacketQueue)).start()
        Thread(NetReader(inputStream, activeRequests)).start()
    }

    fun request(requestId: Long, generatorId: Long, x: Int, z: Int): Chunk {
        val queue = LinkedBlockingQueue<Chunk>()
        synchronized(activeRequests) {
            activeRequests[requestId] = queue
        }

        outboundPacketQueue.put(
            GenerateChunk(
                requestId,
                generatorId,
                x,
                z
            )
        )

        val chunk = queue.take()
        synchronized(activeRequests) {
            activeRequests.remove(requestId)
        }
        return chunk
    }
}

class MyGenerator(val plugin: JavaPlugin, address: String, port: UShort) : ChunkGenerator() {
    private val socket = Socket(address, port.toInt())

    private val networker = Networker(socket.getInputStream(), socket.getOutputStream())

    private var generatorId: Long? = null

    init {
        if (!socket.isConnected) {
            throw Exception("massive fail! socket not connected!")
        }

        val packet = AddGenerator(
            10,
            "BIG_FART",
            -64,
            320,
            0
        )

        val buf = packet.toBytes().toByteArray()

        synchronized(socket) {
            println("writing buffer to socket")
            socket.getOutputStream()!!.write(buf)
            println("finished writing buffer")

            println("trying to read response from server...")
            val compressedSize = socket.getInputStream().readNBytes(4).getUIntAt(0)
            println("got packet with compressed size of $compressedSize")
            val decompressedSize = socket.getInputStream().readNBytes(4).getUIntAt(0)
            val compressedBuffer = socket.getInputStream().readNBytes(compressedSize.toInt())
            println("got response, decoding and casting...")

            val ack = NativeBindings.decodePacket(compressedBuffer, decompressedSize.toLong()) as ConfirmGeneratorAddition
            generatorId = ack.generatorId
        }
    }

    fun start() {
        networker.start()
        logger.info("starting generator client")
    }

    override fun generateBedrock(worldInfo: WorldInfo, random: Random, x: Int, z: Int, chunkData: ChunkData) {
        val requestId = run {
            var id = Random().nextInt()
            while (id < 0) {
                id = Random().nextInt()
            }
            id.toLong()
        }

        val chunk = networker.request(requestId, generatorId!!, x, z)

        for (cx in 0 until 16) {
            for (cz in 0 until 16) {
                for (cy in -64 until 320) {
                    val sectionIdx = (cy - -64) / 16;
                    val sectionspaceY = (cy - -64) % 16;

                    val section = chunk.sections[sectionIdx]
                    val block = if (section.internal == null) {
                        Material.AIR
                    } else {
                        when (section.internal[cx][sectionspaceY][cz]) {
                            0 -> Material.AIR
                            1 -> Material.STONE
                            else -> Material.AIR
                        }
                    }

                    chunkData.setBlock(cx, cy, cz, block)
                }
            }
        }
    }
}

fun ByteArray.getUIntAt(idx: Int) =
    ((this[idx].toUInt() and 0xFFu) shl 24) or
            ((this[idx + 1].toUInt() and 0xFFu) shl 16) or
            ((this[idx + 2].toUInt() and 0xFFu) shl 8) or
            (this[idx + 3].toUInt() and 0xFFu)

fun Double.frac(): Double {
    return this - floor(this)
}

fun Int.clamp(range: IntRange): Int {
    return when {
        this < range.first -> range.first
        this > range.last -> range.last
        else -> this
    }
}

fun ChunkGenerator.ChunkData.fillYRange(x: Int, z: Int, range: IntRange, block: BlockData) {
    for (y in range) {
        this.setBlock(x, y, z, block)
    }
}

fun sigmoid(x: Double): Double {
    return 1.0 / (1.0 + E.pow(-x))
}