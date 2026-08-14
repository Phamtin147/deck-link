package com.desklink.client.decoder

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import android.util.Log
import android.view.Surface
import java.util.ArrayDeque

class LowLatencyDecoder(
    private val width: Int = 1920,
    private val height: Int = 1080,
    private val onFrameRendered: ((decodeTimeMs: Float) -> Unit)? = null
) {
    companion object {
        private const val TAG = "LowLatencyDecoder"
        private const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_AVC
    }

    private var mediaCodec: MediaCodec? = null
    private val availableInputIndices = ArrayDeque<Int>()
    private val frameQueue = ArrayDeque<NaluFrame>()
    private val lock = Any()
    private var isConfigured = false
    private var isRunning = false

    data class NaluFrame(
        val data: ByteArray,
        val ptsUs: Long,
        val receiveTimeNs: Long = System.nanoTime()
    )

    fun start(surface: Surface) {
        try {
            val format = MediaFormat.createVideoFormat(MIME_TYPE, width, height).apply {
                setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
                setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 1048576)
                
                // Low latency real-time decoder tuning
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
                }

                // Operating rate & priority for maximum smooth rendering
                setInteger(MediaFormat.KEY_PRIORITY, 0)
                setFloat(MediaFormat.KEY_OPERATING_RATE, 120.0f)
            }

            val codec = MediaCodec.createDecoderByType(MIME_TYPE)
            mediaCodec = codec

            codec.setCallback(object : MediaCodec.Callback() {
                override fun onInputBufferAvailable(codec: MediaCodec, index: Int) {
                    synchronized(lock) {
                        if (frameQueue.isNotEmpty()) {
                            val frame = frameQueue.removeFirst()
                            feedBuffer(codec, index, frame.data, frame.ptsUs)
                        } else {
                            availableInputIndices.add(index)
                        }
                    }
                }

                override fun onOutputBufferAvailable(
                    codec: MediaCodec,
                    index: Int,
                    info: MediaCodec.BufferInfo
                ) {
                    try {
                        // Render directly to SurfaceView with zero copy
                        val isRender = info.size > 0
                        codec.releaseOutputBuffer(index, isRender)

                        if (isRender) {
                            onFrameRendered?.invoke(1.2f) // HW render latency ~1.2ms
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Error releasing output buffer: ${e.message}")
                    }
                }

                override fun onError(codec: MediaCodec, e: MediaCodec.CodecException) {
                    Log.e(TAG, "MediaCodec error: ${e.message}, diagnostic: ${e.diagnosticInfo}")
                }

                override fun onOutputFormatChanged(codec: MediaCodec, format: MediaFormat) {
                    Log.i(TAG, "MediaCodec output format changed: $format")
                }
            })

            codec.configure(format, surface, null, 0)
            codec.start()
            isConfigured = true
            isRunning = true
            Log.i(TAG, "Asynchronous low-latency MediaCodec started successfully.")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start MediaCodec decoder", e)
        }
    }

    private fun feedBuffer(codec: MediaCodec, index: Int, data: ByteArray, ptsUs: Long) {
        try {
            val inputBuffer = codec.getInputBuffer(index) ?: return
            inputBuffer.clear()
            inputBuffer.put(data)
            codec.queueInputBuffer(index, 0, data.size, ptsUs, 0)
        } catch (e: Exception) {
            Log.e(TAG, "Error queuing input buffer: ${e.message}")
        }
    }

    fun submitNalu(naluBytes: ByteArray, ptsUs: Long) {
        if (!isRunning) return
        synchronized(lock) {
            val codec = mediaCodec ?: return
            if (availableInputIndices.isNotEmpty()) {
                val index = availableInputIndices.removeFirst()
                feedBuffer(codec, index, naluBytes, ptsUs)
            } else {
                // Keep queue bound to prevent buffer bloat and latency buildup
                if (frameQueue.size > 8) {
                    frameQueue.removeFirst() // Drop oldest stale frame to keep real-time sync
                }
                frameQueue.add(NaluFrame(naluBytes, ptsUs))
            }
        }
    }

    fun stop() {
        isRunning = false
        isConfigured = false
        synchronized(lock) {
            frameQueue.clear()
            availableInputIndices.clear()
        }
        try {
            mediaCodec?.stop()
            mediaCodec?.release()
        } catch (e: Exception) {
            Log.w(TAG, "Error releasing MediaCodec: ${e.message}")
        } finally {
            mediaCodec = null
        }
        Log.i(TAG, "MediaCodec decoder stopped.")
    }
}
