package org.routeloom.routeloom

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var pending: MethodChannel.Result? = null
    private var profile: String? = null
    override fun configureFlutterEngine(engine: FlutterEngine) {
        super.configureFlutterEngine(engine)
        MethodChannel(engine.dartExecutor.binaryMessenger, "org.routeloom/vpn").setMethodCallHandler { call, result ->
            when (call.method) {
                "status" -> result.success(RouteLoomVpnService.state)
                "disconnect" -> {
                    pending?.error("cancelled", "VPN start cancelled", null)
                    pending = null; profile = null
                    startService(Intent(this, RouteLoomVpnService::class.java).setAction("stop"))
                    result.success(null)
                }
                "connect" -> {
                    if (pending != null || RouteLoomVpnService.state in listOf("connecting", "connected")) {
                        result.error("busy", "VPN operation in progress", null)
                    } else {
                        val value = call.arguments as? String
                        if (value == null || value.length > 65536) result.error("profile", "Invalid profile", null)
                        else {
                            profile = value
                            pending = result
                            val consent = VpnService.prepare(this)
                            if (consent != null) startActivityForResult(consent, 701) else startVpn()
                        }
                    }
                }
                else -> result.notImplemented()
            }
        }
    }
    private fun startVpn() {
        if (profile == null || pending == null) return
        try {
            RouteLoomVpnService.markStarting()
            startForegroundService(Intent(this, RouteLoomVpnService::class.java).putExtra("profile", profile))
            pending?.success(null)
        } catch (_: Exception) {
            RouteLoomVpnService.markFailed()
            pending?.error("vpn", "Unable to start VPN service", null)
        }
        pending = null
        profile = null
    }
    @Deprecated("Android activity result bridge")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == 701) {
            if (resultCode == Activity.RESULT_OK) startVpn() else {
                pending?.error("permission", "VPN permission declined", null)
                pending = null; profile = null
            }
        }
    }
    override fun onDestroy() {
        pending?.error("cancelled", "Activity closed", null)
        pending = null; profile = null
        super.onDestroy()
    }
}
