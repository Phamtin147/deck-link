package com.desklink.client.input

import android.view.MotionEvent
import android.view.View
import com.desklink.client.protocol.DeskLinkProtocol
import java.io.OutputStream

class TouchInputHandler(
    private val sendTouchPacket: (ByteArray) -> Unit
) : View.OnTouchListener {

    private val touchBuffer = ByteArray(DeskLinkProtocol.TOUCH_PACKET_SIZE)

    override fun onTouch(v: View, event: MotionEvent): Boolean {
        val width = v.width.toFloat()
        val height = v.height.toFloat()
        if (width <= 0f || height <= 0f) return true

        val actionMasked = event.actionMasked
        val actionIndex = event.actionIndex

        when (actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val pointerId = event.getPointerId(actionIndex)
                if (pointerId < 10) {
                    val normX = event.getX(actionIndex) / width
                    val normY = event.getY(actionIndex) / height
                    val pressure = event.getPressure(actionIndex)

                    synchronized(touchBuffer) {
                        DeskLinkProtocol.encodeTouchEvent(
                            pointerId,
                            DeskLinkProtocol.TOUCH_ACTION_DOWN,
                            normX,
                            normY,
                            pressure,
                            touchBuffer
                        )
                        sendTouchPacket(touchBuffer.copyOf())
                    }
                }
            }

            MotionEvent.ACTION_MOVE -> {
                val pointerCount = event.pointerCount.coerceAtMost(10)
                for (i in 0 until pointerCount) {
                    val pointerId = event.getPointerId(i)
                    if (pointerId < 10) {
                        val normX = event.getX(i) / width
                        val normY = event.getY(i) / height
                        val pressure = event.getPressure(i)

                        synchronized(touchBuffer) {
                            DeskLinkProtocol.encodeTouchEvent(
                                pointerId,
                                DeskLinkProtocol.TOUCH_ACTION_MOVE,
                                normX,
                                normY,
                                pressure,
                                touchBuffer
                            )
                            sendTouchPacket(touchBuffer.copyOf())
                        }
                    }
                }
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val pointerId = event.getPointerId(actionIndex)
                if (pointerId < 10) {
                    val normX = event.getX(actionIndex) / width
                    val normY = event.getY(actionIndex) / height
                    val pressure = event.getPressure(actionIndex)

                    synchronized(touchBuffer) {
                        DeskLinkProtocol.encodeTouchEvent(
                            pointerId,
                            DeskLinkProtocol.TOUCH_ACTION_UP,
                            normX,
                            normY,
                            pressure,
                            touchBuffer
                        )
                        sendTouchPacket(touchBuffer.copyOf())
                    }
                }
            }

            MotionEvent.ACTION_CANCEL -> {
                // Release all active touches
                for (i in 0 until event.pointerCount.coerceAtMost(10)) {
                    val pointerId = event.getPointerId(i)
                    synchronized(touchBuffer) {
                        DeskLinkProtocol.encodeTouchEvent(
                            pointerId,
                            DeskLinkProtocol.TOUCH_ACTION_UP,
                            0f,
                            0f,
                            0f,
                            touchBuffer
                        )
                        sendTouchPacket(touchBuffer.copyOf())
                    }
                }
            }
        }

        return true
    }
}
