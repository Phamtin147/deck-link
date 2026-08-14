package com.desklink.client

import android.os.Build
import android.os.Bundle
import android.view.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.desklink.client.databinding.ActivityMainBinding
import com.desklink.client.decoder.LowLatencyDecoder
import com.desklink.client.input.TouchInputHandler
import com.desklink.client.network.DeskLinkClient

class MainActivity : AppCompatActivity(), SurfaceHolder.Callback {

    private lateinit var binding: ActivityMainBinding
    private var decoder: LowLatencyDecoder? = null
    private var client: DeskLinkClient? = null
    private var surfaceAvailable = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Prevent tablet from sleeping during USB display session
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        setupFullscreen()
        setupSurfaceAndTouch()
    }

    private fun setupFullscreen() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        val controller = WindowInsetsControllerCompat(window, window.decorView)
        controller.hide(WindowInsetsCompat.Type.systemBars())
        controller.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    }

    private fun setupSurfaceAndTouch() {
        binding.surfaceView.holder.addCallback(this)

        val touchHandler = TouchInputHandler { packet ->
            client?.sendTouchPacket(packet)
        }
        binding.surfaceView.setOnTouchListener(touchHandler)

        binding.btnToggleHud.setOnClickListener {
            binding.statsHud.visibility = if (binding.statsHud.visibility == View.VISIBLE) {
                View.GONE
            } else {
                View.VISIBLE
            }
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        surfaceAvailable = true
        initStreamingPipeline(holder.surface)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {}

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        surfaceAvailable = false
        stopStreamingPipeline()
    }

    private fun initStreamingPipeline(surface: Surface) {
        val displayMetrics = resources.displayMetrics
        val streamWidth = maxOf(displayMetrics.widthPixels, displayMetrics.heightPixels)
        val streamHeight = minOf(displayMetrics.widthPixels, displayMetrics.heightPixels)
        val refreshRate = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            display?.mode?.refreshRate?.toInt() ?: 60
        } else {
            @Suppress("DEPRECATION")
            windowManager.defaultDisplay.refreshRate.toInt()
        }
        val densityDpi = displayMetrics.densityDpi

        decoder = LowLatencyDecoder(streamWidth, streamHeight) { _ ->
            // Optional latency telemetry callback
        }
        decoder?.start(surface)

        client = DeskLinkClient(
            host = "127.0.0.1",
            port = 9999,
            deviceWidth = streamWidth,
            deviceHeight = streamHeight,
            deviceFps = refreshRate,
            deviceDensity = densityDpi,
            onConnectionStateChanged = { state ->
                runOnUiThread {
                    when (state) {
                        DeskLinkClient.State.CONNECTING -> {
                            binding.statusOverlay.visibility = View.VISIBLE
                            binding.txtStatusDesc.text = getString(R.string.connecting)
                            binding.statsHud.visibility = View.GONE
                            binding.btnToggleHud.visibility = View.GONE
                        }
                        DeskLinkClient.State.CONNECTED -> {
                            binding.statusOverlay.visibility = View.GONE
                            binding.statsHud.visibility = View.GONE // Hidden by default for clean secondary display
                            binding.btnToggleHud.visibility = View.VISIBLE
                        }
                        DeskLinkClient.State.DISCONNECTED -> {
                            binding.statusOverlay.visibility = View.VISIBLE
                            binding.txtStatusDesc.text = getString(R.string.waiting_usb)
                            binding.statsHud.visibility = View.GONE
                            binding.btnToggleHud.visibility = View.GONE
                        }
                    }
                }
            },
            onVideoNaluReceived = { nalu, ptsUs ->
                decoder?.submitNalu(nalu, ptsUs)
            }
        )
        client?.start()
    }

    private fun stopStreamingPipeline() {
        client?.stop()
        client = null
        decoder?.stop()
        decoder = null
    }

    override fun onResume() {
        super.onResume()
        setupFullscreen()
        if (surfaceAvailable && client == null) {
            initStreamingPipeline(binding.surfaceView.holder.surface)
        }
    }

    override fun onPause() {
        super.onPause()
        stopStreamingPipeline()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopStreamingPipeline()
    }
}
