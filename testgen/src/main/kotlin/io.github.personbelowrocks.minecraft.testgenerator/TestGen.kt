package io.github.personbelowrocks.minecraft.testgenerator

import com.sk89q.worldedit.IncompleteRegionException
import com.sk89q.worldedit.WorldEdit
import com.sk89q.worldedit.bukkit.BukkitAdapter
import com.sk89q.worldedit.regions.CuboidRegion
import org.bukkit.Bukkit
import org.bukkit.command.Command
import org.bukkit.command.CommandExecutor
import org.bukkit.command.CommandSender
import org.bukkit.entity.Player
import org.bukkit.plugin.java.JavaPlugin
import com.sk89q.worldedit.math.BlockVector3
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.Socket
import java.nio.BufferUnderflowException
import java.nio.ByteBuffer
import java.nio.ByteOrder


val logger = Bukkit.getLogger()


class Main : JavaPlugin() {
    override fun onEnable() {
        logger.info("AYOOOOO WASSSUP HOMIES!!!! WUSS GOOOOOOOOOOOOOOD!!! LETS PLACE SOME MFING BLOCKS AYYYY!!!")

        logger.info("Starting client...")
        val client = Client("127.0.0.1", 4432)
        client.connect()

        val instance = WorldEdit.getInstance()
        this.server.getPluginCommand("/pbrgen")!!.setExecutor(RootCommand(instance, client))
    }

    override fun onDisable() {
        logger.info("die")
    }
}

class Client(private val address: String?, private val port: Int) {
    private var outputStream: BufferedOutputStream? = null
    private var inputStream: BufferedInputStream? = null

    fun connect() {
        val socket = Socket(address, port)

        outputStream = BufferedOutputStream(socket.getOutputStream())
        inputStream = BufferedInputStream(socket.getInputStream())
    }

    fun writePacket(packet: UpstreamPacket) {
        outputStream!!.write(packet.toBytes().toByteArray())
        outputStream!!.flush()
    }

    fun readPacket(): UpstreamPacket? {

        val compressedLen = try {
            ByteBuffer.wrap(inputStream!!.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int
        } catch (e: BufferUnderflowException) {
            return null
        }

        val decompressedLen = try {
            ByteBuffer.wrap(inputStream!!.readNBytes(4)).order(ByteOrder.BIG_ENDIAN).int
        } catch (e: BufferUnderflowException) {
            return null
        }

        val packetBuffer = inputStream!!.readNBytes(compressedLen)
        return NativeBindings.decodePacket(packetBuffer, decompressedLen.toLong()) as? UpstreamPacket
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

        return true
    }
}
