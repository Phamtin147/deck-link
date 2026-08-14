package com.desklink.client.protocol

import java.nio.ByteBuffer
import java.nio.ByteOrder

object DeskLinkProtocol {
    const val MAGIC_BYTE: Byte = 0x44 // 'D'
    const val PAYLOAD_TYPE_VIDEO: Byte = 0x01
    const val EVENT_TYPE_TOUCH: Byte = 0x02
    const val EVENT_TYPE_CONFIG: Byte = 0x03

    const val VIDEO_HEADER_SIZE = 14
    const val TOUCH_PACKET_SIZE = 15
    const val CONFIG_PACKET_SIZE = 15

    const val TOUCH_ACTION_DOWN: Byte = 0x00
    const val TOUCH_ACTION_MOVE: Byte = 0x01
    const val TOUCH_ACTION_UP: Byte = 0x02

    data class VideoHeader(
        val magic: Byte,
        val payloadType: Byte,
        val payloadLength: Int,
        val ptsUs: Long
    )

    fun decodeVideoHeader(buffer: ByteBuffer): VideoHeader? {
        if (buffer.remaining() < VIDEO_HEADER_SIZE) return null
        buffer.order(ByteOrder.BIG_ENDIAN)

        val magic = buffer.get()
        if (magic != MAGIC_BYTE) return null

        val payloadType = buffer.get()
        if (payloadType != PAYLOAD_TYPE_VIDEO) return null

        val payloadLength = buffer.int
        val ptsUs = buffer.long

        return VideoHeader(magic, payloadType, payloadLength, ptsUs)
    }

    fun encodeConfigPacket(
        width: Int,
        height: Int,
        fps: Int,
        densityDpi: Int,
        targetBuf: ByteArray
    ) {
        targetBuf[0] = EVENT_TYPE_CONFIG
        // Width (2 bytes or 4 bytes)
        targetBuf[1] = (width ushr 24).toByte()
        targetBuf[2] = (width ushr 16).toByte()
        targetBuf[3] = (width ushr 8).toByte()
        targetBuf[4] = width.toByte()

        // Height (4 bytes)
        targetBuf[5] = (height ushr 24).toByte()
        targetBuf[6] = (height ushr 16).toByte()
        targetBuf[7] = (height ushr 8).toByte()
        targetBuf[8] = height.toByte()

        // FPS (2 bytes)
        targetBuf[9] = (fps ushr 8).toByte()
        targetBuf[10] = fps.toByte()

        // Density DPI (2 bytes)
        targetBuf[11] = (densityDpi ushr 8).toByte()
        targetBuf[12] = densityDpi.toByte()

        targetBuf[13] = 0
        targetBuf[14] = 0
    }

    fun encodeTouchEvent(
        pointerId: Int,
        action: Byte,
        normX: Float,
        normY: Float,
        pressure: Float,
        targetBuf: ByteArray
    ) {
        val clampedX = normX.coerceIn(0f, 1f)
        val clampedY = normY.coerceIn(0f, 1f)
        val clampedP = pressure.coerceIn(0f, 1f)

        targetBuf[0] = EVENT_TYPE_TOUCH
        targetBuf[1] = (pointerId and 0xFF).toByte()
        targetBuf[2] = action

        val xBits = java.lang.Float.floatToIntBits(clampedX)
        val yBits = java.lang.Float.floatToIntBits(clampedY)
        val pBits = java.lang.Float.floatToIntBits(clampedP)

        // Big-Endian packing
        targetBuf[3] = (xBits ushr 24).toByte()
        targetBuf[4] = (xBits ushr 16).toByte()
        targetBuf[5] = (xBits ushr 8).toByte()
        targetBuf[6] = xBits.toByte()

        targetBuf[7] = (yBits ushr 24).toByte()
        targetBuf[8] = (yBits ushr 16).toByte()
        targetBuf[9] = (yBits ushr 8).toByte()
        targetBuf[10] = yBits.toByte()

        targetBuf[11] = (pBits ushr 24).toByte()
        targetBuf[12] = (pBits ushr 16).toByte()
        targetBuf[13] = (pBits ushr 8).toByte()
        targetBuf[14] = pBits.toByte()
    }
}
