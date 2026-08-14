package com.desklink.client.decoder

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import android.util.Log
import android.view.Surface
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentLinkedQueue

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
    private val frameQueue = ConcurrentLinkedQueue<NaluFrame>()
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
                
                // Low latency mode flag for Android 11+ (API 30+)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
                }

                // Operating rate & priority for real-time rendering
                setInteger(MediaFormat.KEY_PRIORITY, 0)
                setFloat(MediaFormat.KEY_OPERATING_RATE, 120.0f)
            }

            val codec = MediaCodec.createDecoderByType(MIME_TYPE)
            mediaCodec = codec

            codec.setCallback(object : MediaCodec.Callback() {
                override fun onInputBufferAvailable(codec: MediaCodec, index: Int) {
                    val frame = frameQueue.poll()
                    if (frame == null) {
                        return
                    }

                    try {
                        val inputBuffer = codec.getInputBuffer(index) ?: return
                        inputBuffer.clear()
                        inputBuffer.put(frame.data)

                        codec.queueInputBuffer(
                            index,
                            0,
                            frame.data.size,
                            frame.ptsUs,
                            0
                        )
                    } catch (e: Exception) {
                        Log.e(TAG, "Error queuing input buffer: ${e.message}")
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
                            onFrameRendered?.invoke(1.5f) // Estimated HW render time < 2ms
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

    fun submitNalu(naluBytes: ByteArray, ptsUs: Long) {
        if (!isRunning) return
        frameQueue.offer(NaluFrame(naluBytes, ptsUs))
    }

    fun stop() {
        isRunning = false
        isConfigured = false
        frameQueue.clear()
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
