package org.routeloom.routeloom
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.system.Os
import android.system.OsConstants
import android.system.StructPollfd
import org.json.JSONObject
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.InetAddress
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
class RouteLoomVpnService : VpnService() {
    companion object {
        @Volatile var state = "disconnected"; private set
        @Volatile var lockdown = false; private set
        @Volatile private var activeEngine = 0L
        fun snapshot(): String = NativeEngine.snapshot(activeEngine)
        fun command(command: String): Boolean = NativeEngine.command(activeEngine, command)
        fun markStarting() { state = "connecting" }
        fun markFailed() { state = "failed" }
    }
    private var session: Session? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        lockdown = isAlwaysOn && isLockdownEnabled
        if (intent?.action == "stop") { stopVpn(); return START_NOT_STICKY }
        if (session != null) {
            if (intent?.hasExtra("profile") == true && session!!.unavailable()) { session!!.close(); session = null }
            else return START_STICKY
        }
        val raw = try { intent?.getStringExtra("profile") ?: ProfileVault.load(this) } catch (_: Exception) { null }
        if (raw == null) { state = "failed"; stopSelf(); return START_NOT_STICKY }
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("vpn", "RouteLoom VPN", NotificationManager.IMPORTANCE_LOW))
        val stop = android.app.PendingIntent.getService(this, 1, Intent(this,RouteLoomVpnService::class.java).setAction("stop"), android.app.PendingIntent.FLAG_UPDATE_CURRENT or android.app.PendingIntent.FLAG_IMMUTABLE)
        val open = android.app.PendingIntent.getActivity(this, 2, Intent(this,MainActivity::class.java), android.app.PendingIntent.FLAG_UPDATE_CURRENT or android.app.PendingIntent.FLAG_IMMUTABLE)
        val notification = Notification.Builder(this,"vpn").setContentTitle("RouteLoom")
            .setContentIntent(open).addAction(Notification.Action.Builder(null,getString(R.string.vpn_disconnect),stop).build())
            .setContentText(getString(R.string.vpn_active)).setSmallIcon(android.R.drawable.ic_lock_lock).setOngoing(true).build()
        startForeground(1, notification)
        state = "connecting"
        session = Session(raw).also { it.start() }
        return START_STICKY
    }
    private fun stopVpn() { session?.close(); session = null; state = "disconnected"; stopForeground(STOP_FOREGROUND_REMOVE); stopSelf() }
    override fun onRevoke() { try { ProfileVault.clear(this) } catch (_: Exception) {} finally { stopVpn(); super.onRevoke() } }
    override fun onDestroy() { lockdown = false; session?.close(); session = null; if (state != "failed") state = "disconnected"; super.onDestroy() }
    private inner class Session(private val raw: String) {
        private val closed = AtomicBoolean(false)
        private val workers = Executors.newFixedThreadPool(2)
        @Volatile private var handle = 0L
        @Volatile private var tun: ParcelFileDescriptor? = null
        fun unavailable() = NativeEngine.status(handle) == 2
        fun start() { workers.execute { try { run() } catch (_: Exception) { fail() } } }
        private fun run() {
            require(raw.length <= 65536)
            handle = NativeEngine.start(raw, this@RouteLoomVpnService)
            check(handle != 0L)
            synchronized(this) {
                if (closed.get()) { NativeEngine.stop(handle); return }
                ProfileVault.save(this@RouteLoomVpnService, raw)
                activeEngine = handle
            }
            val p = JSONObject(raw)
            val builder = Builder().setSession("RouteLoom").setMtu(1280).setBlocking(true).setMetered(false)
                .addAddress(InetAddress.getByName(p.getString("ipv4")),24).addAddress(InetAddress.getByName(p.getString("ipv6")),64)
            val routes = p.getJSONArray("routes")
            for (i in 0 until routes.length()) {
                val route = routes.getString(i).split('/')
                builder.addRoute(InetAddress.getByName(route[0]),route[1].toInt())
            }
            builder.addRoute("198.18.0.0",16).addRoute("fdfe::",112)
            val dns = p.getJSONArray("dns")
            for (i in 0 until dns.length()) builder.addDnsServer(InetAddress.getByName(dns.getString(i)))
            val fd = builder.establish() ?: error("VPN permission revoked")
            tun = fd
            if (closed.get()) { fd.close(); return }
            workers.execute {
                try {
                    val source = FileInputStream(fd.fileDescriptor)
                    val poll = StructPollfd().apply { this.fd = fd.fileDescriptor; events = OsConstants.POLLIN.toShort() }
                    val bytes = ByteArray(1281)
                    while (!closed.get()) {
                        if (Os.poll(arrayOf(poll),500) == 0) continue
                        val n = source.read(bytes)
                        if (n <= 0) error("TUN closed")
                        if (n <= 1280) NativeEngine.write(handle,bytes.copyOf(n))
                    }
                } catch (_: Exception) { fail() }
            }
            val sink = FileOutputStream(fd.fileDescriptor)
            while (!closed.get()) {
                if (NativeEngine.status(handle) == 2) { state = "blocked"; Thread.sleep(200); continue }
                val packet = NativeEngine.read(handle)
                if (packet != null && packet.isNotEmpty()) sink.write(packet)
                state = when (NativeEngine.status(handle)) { 1 -> "connected"; 0 -> "reconnecting"; else -> "blocked" }
                lockdown = isAlwaysOn && isLockdownEnabled
                // Keep capture active on engine failure; system lockdown covers process death.
            }
        }
        private fun fail() {
            if (closed.get()) return
            close()
            android.os.Handler(mainLooper).post { if (session === this) { session = null; state = "failed"; stopForeground(STOP_FOREGROUND_REMOVE); stopSelf() } }
        }
        fun close() { synchronized(this) {
            if (!closed.compareAndSet(false,true)) return
            try { tun?.close() } catch (_: Exception) {}
            if (activeEngine == handle) activeEngine = 0L
            NativeEngine.stop(handle); workers.shutdownNow()
        } }
    }
}
