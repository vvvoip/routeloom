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
                "updateDirectory" -> {
                    val folder = java.io.File(filesDir,"updates"); folder.mkdirs(); result.success(folder.absolutePath)
                }
                "installUpdate" -> {
                    try {
                        val file = java.io.File(call.arguments as String).canonicalFile
                        val directory = java.io.File(filesDir,"updates").canonicalFile
                        require(file.parentFile == directory && file.name == "routeloom-android.apk" && file.isFile)
                        if (!packageManager.canRequestPackageInstalls()) {
                            startActivity(Intent(android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,android.net.Uri.parse("package:"+packageName)))
                            result.error("permission","Allow RouteLoom package installation, then try again",null)
                        } else {
                            val uri = androidx.core.content.FileProvider.getUriForFile(this,packageName+".updates",file)
                            startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
                            result.success(null)
                        }
                    } catch (_: Exception) { result.error("update","Unable to open verified update",null) }
                }
                "forgetProfile" -> {
                    try {
                        check(pending == null && RouteLoomVpnService.state !in listOf("connecting","reconnecting","connected","blocked"))
                        val requested = org.json.JSONObject(call.arguments as String)
                        val saved = ProfileVault.load(this)?.let { org.json.JSONObject(it) }
                        if (saved != null && saved.getString("device") == requested.getString("device") && saved.getString("endpoint") == requested.getString("endpoint")) ProfileVault.clear(this)
                        result.success(null)
                    } catch (_: Exception) { result.error("profile","Unable to remove background profile",null) }
                }
                "status" -> result.success(RouteLoomVpnService.state)
                "guardStatus" -> result.success(RouteLoomVpnService.lockdown)
                "guardSettings" -> { startActivity(Intent(android.provider.Settings.ACTION_VPN_SETTINGS)); result.success(null) }
                "disconnect" -> {
                    pending?.error("cancelled", "VPN start cancelled", null)
                    pending = null; profile = null
                    startService(Intent(this, RouteLoomVpnService::class.java).setAction("stop"))
                    result.success(null)
                }
                "connect" -> {
                    if (pending != null || RouteLoomVpnService.state in listOf("connecting", "reconnecting", "connected")) {
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
