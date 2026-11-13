package org.routeloom.routeloom
import android.net.VpnService
object NativeEngine {
    init { System.loadLibrary("routeloom_engine") }
    @JvmStatic external fun start(profile: String, service: VpnService): Long
    @JvmStatic external fun write(handle: Long, packet: ByteArray)
    @JvmStatic external fun read(handle: Long): ByteArray?
    @JvmStatic external fun status(handle: Long): Int
    @JvmStatic external fun stop(handle: Long)
}
