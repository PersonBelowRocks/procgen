package io.github.personbelowrocks.minecraft.testgenerator

import com.sk89q.worldedit.EditSession
import com.sk89q.worldedit.IncompleteRegionException
import com.sk89q.worldedit.WorldEdit
import com.sk89q.worldedit.bukkit.BukkitAdapter
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
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.*
import java.util.concurrent.ArrayBlockingQueue

val logger = Bukkit.getLogger()



class Main : JavaPlugin() {
    private var client: Client? = null

    override fun onEnable() {
        logger.info("Starting...")

        logger.info("Starting client...")
        client = Client(Connection("127.0.0.1", 4432))
        client?.run(this)

        val instance = WorldEdit.getInstance()
        this.server.getPluginCommand("/pbrgen")!!.setExecutor(RootCommand(instance, client!!))
    }

    override fun onDisable() {
        client?.stop()
        logger.info("Stopping...")
    }
}

abstract class IoWorker: Runnable {
    private var running = true

    fun stop() {
        running = false
    }

    override fun run() {
        while (this.running) {
            tick()
        }
    }

    abstract fun tick()
}

class InnerReader(private val queue: Queue<DownstreamPacket>, private val stream: BufferedInputStream): IoWorker() {
    override fun tick() {
        val packetSize = ByteBuffer.wrap(stream.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int
        val sizeHint = ByteBuffer.wrap(stream.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int

        val packetBuffer = stream.readNBytes(packetSize)

        val packet = DownstreamPacket.decode(packetBuffer, sizeHint.toLong())
        queue.add(packet)
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
        stream.write(buf)
        stream.flush()
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
    private val reader: Reader
    private val writer: Writer

    init {
        val socket = Socket(address, port)

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
    fun packet(requestId: Long): UpstreamPacket
}

class RegionRequest(private val region: CuboidRegion, private val session: EditSession, private val name: String): TransmittableRequest {
    // val extent: Extent

    override fun packet(requestId: Long): UpstreamPacket {
        return GenerateRegion(
            requestId,
            region.pos1,
            region.pos2,
            name
        )
    }

    override fun feed(chunk: Chunk) {
        for (x in chunk.min.x until chunk.max.x) {
            for (y in chunk.min.y until chunk.max.y) {
                for (z in chunk.min.z until chunk.max.z) {
                    val pos = BlockVector3.at(x, y, z).add(region.minimumPoint)

                    val type = when (chunk.storage[x][y][z]) {
                        0L -> BlockTypes.AIR
                        1L -> BlockTypes.STONE
                        else -> BlockTypes.RED_STAINED_GLASS
                    }!!

                    session.setBlock(pos, type.defaultState)
                }
            }
        }
    }

    override fun finish() {
        session.close()
    }
}

private class ClientLoop(private val connection: Connection, private val requests: MutableMap<Long, FeedableRequest>): BukkitRunnable() {
    override fun run() {
        val receivedPackets = mutableListOf<DownstreamPacket>()

        while (true)
            receivedPackets.add(connection.readPacket() ?: break)

        // TODO: we should try to spread feeding across several ticks instead of flooding a single tick
        for (packet in receivedPackets.map {it as DownstreamResponse}) {
            when (packet) {
                is VoxelData -> requests[packet.requestId]?.feed(packet.chunk)
                is FinishRequest -> requests.remove(packet.requestId)?.finish()
                else -> TODO()
            }
        }
    }
}

class Client(private val connection: Connection) {
    private val activeVoxelRequests = mutableMapOf<Long, FeedableRequest>()

    private var loop: ClientLoop? = null

    private fun randomRequestId(): Long {
        var requestId: Long
        do {
            requestId = Random().nextLong()
        } while (activeVoxelRequests.containsKey(requestId))

        return requestId
    }

    fun newRequest(request: TransmittableRequest) {
        val requestId = randomRequestId()
        connection.writePacket(request.packet(requestId))
        activeVoxelRequests[requestId] = request
    }

    fun feed(requestId: Long, chunk: Chunk) {
        activeVoxelRequests[requestId]!!.feed(chunk)
    }

    fun finish(requestId: Long) {
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
