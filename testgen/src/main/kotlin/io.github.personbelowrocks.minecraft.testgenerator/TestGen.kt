package io.github.personbelowrocks.minecraft.testgenerator

import com.sk89q.worldedit.EditSession
import com.sk89q.worldedit.IncompleteRegionException
import com.sk89q.worldedit.WorldEdit
import com.sk89q.worldedit.bukkit.BukkitAdapter
import com.sk89q.worldedit.extension.input.ParserContext
import com.sk89q.worldedit.math.BlockVector3
import com.sk89q.worldedit.regions.CuboidRegion
import com.sk89q.worldedit.world.block.BlockTypes
import io.github.personbelowrocks.minecraft.testgenerator.packets.*
import org.bukkit.Bukkit
import org.bukkit.command.Command
import org.bukkit.command.CommandExecutor
import org.bukkit.command.CommandSender
import org.bukkit.entity.Player
import org.bukkit.plugin.java.JavaPlugin
import org.bukkit.scheduler.BukkitRunnable
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.Socket
import java.net.SocketException
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.*
import java.util.concurrent.ArrayBlockingQueue

val logger = Bukkit.getLogger()

class Main : JavaPlugin() {
    private var client: Client? = null

    override fun onEnable() {
        logger.info("Starting...")

        val instance = WorldEdit.getInstance()

        val ctx = ParserContext()
        ctx.actor = BukkitAdapter.adapt(this.server.consoleSender)
        val block = instance.blockFactory.parseFromInput("stone_slab[type=top]", ctx)
        logger.info("Block: $block")

        logger.info("Starting client...")
        client = Client(Connection("127.0.0.1", 4432))
        client?.run(this)

        this.server.getPluginCommand("/pbrgen")!!.setExecutor(RootCommand(instance, client!!))
    }

    override fun onDisable() {
        client?.stop()
        logger.info("Stopping...")
    }
}

class StopWorker: Throwable()

abstract class IoWorker: Runnable {
    private var running = true

    fun stop() {
        running = false
    }

    override fun run() {
        while (this.running) {
            try {
                tick()
            } catch (e: StopWorker) {
                this.running = false
            }
        }
    }

    @Throws(StopWorker::class)
    abstract fun tick()
}

class InnerReader(private val queue: Queue<DownstreamPacket>, private val stream: BufferedInputStream): IoWorker() {
    override fun tick() {
        try {

            val packetSize = ByteBuffer.wrap(stream.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int
            val sizeHint = ByteBuffer.wrap(stream.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int


            val packetBuffer = stream.readNBytes(packetSize)

            val packet = DownstreamPacket.decode(packetBuffer, sizeHint.toLong())

            queue.add(packet)

        } catch (e: SocketException) {
            logger.info("Reader was disconnected from the server.")
            throw StopWorker()
        }
    }
}

class Reader(private val queue: Queue<DownstreamPacket>, stream: BufferedInputStream) {
    private val inner = InnerReader(queue, stream)
    private val thread: Thread = Thread(inner, "READER_THREAD")

    init {
        thread.start()
    }

    fun read(): DownstreamPacket? {
        return queue.poll()
    }

    fun stop() {
        inner.stop()
        thread.join()
    }
}

class InnerWriter(private val queue: Queue<UpstreamPacket>, private val stream: BufferedOutputStream): IoWorker() {
    override fun tick() {
        val packet = queue.poll() ?: run {
            Thread.sleep(10)
            return
        }

        val buf = packet.toBytes()

        try {
            stream.write(buf)
            stream.flush()
        } catch (e: SocketException) {
            logger.info("Writer was disconnected from the server.")
            throw StopWorker()
        }
    }
}

class Writer(private val queue: Queue<UpstreamPacket>, stream: BufferedOutputStream) {
    private val inner = InnerWriter(queue, stream)
    private val thread: Thread = Thread(inner, "WRITER_THREAD")

    init {
        thread.start()
    }

    fun send(packet: UpstreamPacket) {
        queue.add(packet)
    }

    fun stop() {
        inner.stop()
        thread.join()
    }
}

class Connection(address: String?, port: Int) {
    private val socket: Socket

    private val reader: Reader
    private val writer: Writer

    init {
        socket = Socket(address, port)

        val inputStream = BufferedInputStream(socket.getInputStream())
        val readerQueue = ArrayBlockingQueue<DownstreamPacket>(32)

        reader = Reader(readerQueue, inputStream)

        val outputStream = BufferedOutputStream(socket.getOutputStream())
        val writerQueue = ArrayBlockingQueue<UpstreamPacket>(32)

        writer = Writer(writerQueue, outputStream)
    }

    fun writePacket(packet: UpstreamPacket) {
        writer.send(packet)
    }

    fun readPacket(): DownstreamPacket? {
        return reader.read()
    }

    fun stop() {
        socket.close()

        synchronized(reader) {
            reader.stop()
        }
        synchronized(writer) {
            writer.stop()
        }
    }
}

interface FeedableRequest {
    fun feed(chunk: Chunk)
    fun finish()
}
interface TransmittableRequest: FeedableRequest {
    fun packet(requestId: Int): UpstreamPacket
}

class RegionRequest(private val region: CuboidRegion, private val session: EditSession, private val name: String): TransmittableRequest {

    override fun packet(requestId: Int): UpstreamPacket {
        return GenerateRegion(
            requestId,
            region.pos1,
            region.pos2,
            name
        )
    }

    override fun feed(chunk: Chunk) {
        logger.info("Request at region $region was fed chunk [${chunk.min}..${chunk.max}]")

        for (x in 0 until 16) {
            for (y in 0 until 16) {
                for (z in 0 until 16) {
                    val pos = BlockVector3.at(x, y, z).add(chunk.min)

                    val block = when (chunk.storage[x][y][z]) {
                        0L -> BlockTypes.AIR!!.defaultState
                        1L -> BlockTypes.STONE!!.defaultState
                        Long.MAX_VALUE -> session.getBlock(pos)
                        else -> BlockTypes.RED_STAINED_GLASS!!.defaultState
                    }!!
                    session.setBlock(pos, block)
                }
            }
        }
    }

    override fun finish() {
        logger.info("Request at region $region was finished")
        session.close()
    }
}

private class ClientLoop(private val connection: Connection, private val requests: MutableMap<Int, FeedableRequest>): BukkitRunnable() {
    override fun run() {
        val receivedPackets = mutableListOf<DownstreamPacket>()

        while (true)
            receivedPackets.add(connection.readPacket() ?: break)

        // TODO: we should try to spread feeding across several ticks instead of flooding a single tick
        for (packet in receivedPackets.map {it as DownstreamResponse}) {
            logger.info("Received response: $packet")

            when (packet) {
                is VoxelData -> {
                    logger.info("Received VoxelData")
                    val request = requests[packet.requestId]

                    if (request == null) logger.info("Request was null")

                    request?.feed(packet.chunk)
                }

                is FinishRequest -> {
                    logger.info("Received FinishRequest")
                    val request = requests[packet.requestId]

                    if (request == null) logger.info("Request was null")

                    request?.finish()
                }
                else -> TODO()
            }
        }
    }
}

class Client(private val connection: Connection) {
    private val activeVoxelRequests = mutableMapOf<Int, FeedableRequest>()

    private var loop: ClientLoop? = null

    private fun randomRequestId(): Int {
        var requestId: Int
        do {
            requestId = Random().nextInt()
        } while (activeVoxelRequests.containsKey(requestId))

        return requestId
    }

    fun newRequest(request: TransmittableRequest) {
        val requestId = randomRequestId()
        activeVoxelRequests[requestId] = request
        connection.writePacket(request.packet(requestId))
    }

    fun feed(requestId: Int, chunk: Chunk) {
        activeVoxelRequests[requestId]!!.feed(chunk)
    }

    fun finish(requestId: Int) {
        activeVoxelRequests[requestId]!!.finish()
    }

    fun run(plugin: JavaPlugin) {
        loop = ClientLoop(connection, activeVoxelRequests)
        loop?.runTaskTimer(plugin, 20, 2)
    }

    fun stop() {
        loop?.cancel()
        connection.stop()
    }
}

class RootCommand(private val we: WorldEdit, private val client: Client) : CommandExecutor {
    override fun onCommand(sender: CommandSender, command: Command, label: String, args: Array<out String>): Boolean {
        val bukkitPlayer = sender as? Player ?: run {
            sender.sendMessage("You must be a player to use this command!")
            return true
        }

        val player = BukkitAdapter.adapt(bukkitPlayer)
        val session = we.sessionManager.get(player) ?: run {
            sender.sendMessage("No session found")
            return true
        }

        val sel = try {
            session.selection as? CuboidRegion ?: run {
                sender.sendMessage("You need a cuboid selection!")
                return true
            }
        } catch (e: IncompleteRegionException) {
            sender.sendMessage("You don't have a valid selection!")
            return true
        }

        sender.sendMessage("your selection is (${sel.pos1}, ${sel.pos2})")

        val buf = args.joinToString("")
        sender.sendMessage(buf)

        val edit = we.newEditSession(player)
        val request = RegionRequest(
            sel,
            edit,
            "DEMO"
        )
        client.newRequest(request)

        return true
    }
}
