package com.desklink.client.network

import android.util.Log
import com.desklink.client.protocol.DeskLinkProtocol
import kotlinx.coroutines.*
import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.ByteBuffer
import java.util.concurrent.LinkedBlockingQueue

class DeskLinkClient(
    private val host: String = "127.0.0.1",
    private val port: Int = 9999,
    private val deviceWidth: Int = 1920,
    private val deviceHeight: Int = 1080,
    private val deviceFps: Int = 60,
    private val deviceDensity: Int = 160,
    private val onConnectionStateChanged: (State) -> Unit,
    private val onVideoNaluReceived: (nalu: ByteArray, ptsUs: Long) -> Unit
) {
    companion object {
        private const val TAG = "DeskLinkClient"
        private const val CONNECT_TIMEOUT_MS = 3000
        private const val RECONNECT_INTERVAL_MS = 2000L
    }

    enum class State {
        CONNECTING,
        CONNECTED,
        DISCONNECTED
    }

    private var clientScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var isRunning = false
    private var activeSocket: Socket? = null
    private val outgoingQueue = LinkedBlockingQueue<ByteArray>(256)

    fun start() {
        if (isRunning) return
        isRunning = true
        clientScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

        clientScope.launch {
            while (isRunning && isActive) {
                try {
                    onConnectionStateChanged(State.CONNECTING)
                    Log.i(TAG, "Attempting TCP connection to $host:$port...")

                    val socket = Socket()
                    socket.tcpNoDelay = true // Disable Nagle's algorithm for zero packet delay
                    socket.receiveBufferSize = 1048576 // 1MB buffer
                    socket.sendBufferSize = 65536
                    socket.connect(InetSocketAddress(host, port), CONNECT_TIMEOUT_MS)

                    activeSocket = socket
                    onConnectionStateChanged(State.CONNECTED)
                    Log.i(TAG, "Connected to DeskLink Host Daemon! Sending device config: ${deviceWidth}x${deviceHeight}@${deviceFps}fps")

                    val inputStream = DataInputStream(BufferedInputStream(socket.getInputStream(), 131072))
                    val outputStream = socket.getOutputStream()

                    // Send initial handshake config packet (device native resolution & aspect ratio)
                    val configBuf = ByteArray(DeskLinkProtocol.CONFIG_PACKET_SIZE)
                    DeskLinkProtocol.encodeConfigPacket(deviceWidth, deviceHeight, deviceFps, deviceDensity, configBuf)
                    outputStream.write(configBuf)
                    outputStream.flush()

                    // Launch touch sender job
                    val senderJob = launch {
                        while (isActive && !socket.isClosed) {
                            val packet = outgoingQueue.take()
                            try {
                                outputStream.write(packet)
                                outputStream.flush()
                            } catch (e: Exception) {
                                break
                            }
                        }
                    }

                    // Main reading loop (Video Frames)
                    val headerBuffer = ByteBuffer.allocate(DeskLinkProtocol.VIDEO_HEADER_SIZE)
                    val headerBytes = ByteArray(DeskLinkProtocol.VIDEO_HEADER_SIZE)

                    while (isActive && !socket.isClosed) {
                        inputStream.readFully(headerBytes)
                        headerBuffer.clear()
                        headerBuffer.put(headerBytes)
                        headerBuffer.flip()

                        val header = DeskLinkProtocol.decodeVideoHeader(headerBuffer)
                        if (header == null) {
                            Log.w(TAG, "Invalid video header magic/type, reconnecting...")
                            break
                        }

                        if (header.payloadLength > 0 && header.payloadLength < 16777216) {
                            val payload = ByteArray(header.payloadLength)
                            inputStream.readFully(payload)
                            onVideoNaluReceived(payload, header.ptsUs)
                        }
                    }

                    senderJob.cancel()
                } catch (e: Exception) {
                    Log.d(TAG, "Socket connection error / disconnected: ${e.message}")
                } finally {
                    try {
                        activeSocket?.close()
                    } catch (_: Exception) {}
                    activeSocket = null
                    onConnectionStateChanged(State.DISCONNECTED)
                }

                if (isRunning && isActive) {
                    delay(RECONNECT_INTERVAL_MS)
                }
            }
        }
    }

    fun sendTouchPacket(packet: ByteArray) {
        outgoingQueue.offer(packet)
    }

    fun stop() {
        isRunning = false
        try {
            activeSocket?.close()
        } catch (_: Exception) {}
        clientScope.cancel()
        outgoingQueue.clear()
        Log.i(TAG, "DeskLinkClient stopped.")
    }
}
