package org.routeloom.routeloom

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.system.Os
import android.system.OsConstants
import android.system.StructPollfd
import org.json.JSONObject
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Socket
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import javax.net.ssl.TrustManagerFactory

class RouteLoomVpnService : VpnService() {
    companion object {
        @Volatile var state = "disconnected"; private set
        fun markStarting() { state = "connecting" }
        fun markFailed() { state = "failed" }
    }
    private var session: Session? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "stop") { stopVpn(); return START_NOT_STICKY }
        if (session != null) return START_NOT_STICKY
        val raw = intent?.getStringExtra("profile")
        if (raw == null) { stopSelf(); return START_NOT_STICKY }
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("vpn", "RouteLoom VPN", NotificationManager.IMPORTANCE_LOW))
        val stop = PendingIntent.getService(this, 0, Intent(this, RouteLoomVpnService::class.java).setAction("stop"), PendingIntent.FLAG_IMMUTABLE)
        val notification = Notification.Builder(this, "vpn").setContentTitle("RouteLoom")
            .setContentText(getString(R.string.vpn_active)).setSmallIcon(android.R.drawable.ic_lock_lock)
            .setOngoing(true).addAction(Notification.Action.Builder(null, getString(R.string.vpn_disconnect), stop).build()).build()
        startForeground(1, notification)
        state = "connecting"
        session = Session(raw).also { it.start() }
        return START_NOT_STICKY
    }
    private fun stopVpn() { session?.close(); session = null; state = "disconnected"; stopForeground(STOP_FOREGROUND_REMOVE); stopSelf() }
    override fun onRevoke() { stopVpn(); super.onRevoke() }
    override fun onDestroy() { session?.close(); session = null; if (state != "failed") state = "disconnected"; super.onDestroy() }

    private inner class Session(private val raw: String) {
        private val closed = AtomicBoolean(false)
        private val workers = Executors.newFixedThreadPool(2)
        private val heartbeat = Executors.newSingleThreadScheduledExecutor()
        @Volatile private var socket: Socket? = null
        @Volatile private var tun: ParcelFileDescriptor? = null
        private var output: DataOutputStream? = null
        @Synchronized private fun frame(kind: Int, data: ByteArray = byteArrayOf()) {
            require(data.size <= 4095)
            output!!.writeInt(data.size + 1); output!!.writeByte(kind); output!!.write(data); output!!.flush()
        }
        private fun read(input: DataInputStream): Pair<Int, ByteArray> {
            val size = input.readInt()
            require(size in 1..4096)
            val kind = input.readUnsignedByte()
            val bytes = ByteArray(size - 1); input.readFully(bytes)
            return Pair(kind, bytes)
        }
        fun start() { workers.execute { try { run() } catch (_: Exception) { fail() } } }
        private fun run() {
            require(raw.length <= 65536)
            val p = JSONObject(raw)
            val device = p.getString("device"); val token = p.getString("token")
            require(device.matches(Regex("[A-Za-z0-9_-]{1,64}")) && token.matches(Regex("[0-9a-f]{64}")))
            val endpoint = p.getString("endpoint")
            val split = endpoint.lastIndexOf(':')
            require(split > 0)
            val host = endpoint.substring(0, split).removePrefix("[").removeSuffix("]")
            require(host.matches(Regex("[0-9a-fA-F:.]+")))
            val port = endpoint.substring(split + 1).toInt(); require(port in 1..65535)
            val ipv4 = InetAddress.getByName(p.getString("ipv4"))
            val ipv6 = InetAddress.getByName(p.getString("ipv6"))
            require(ipv4.address.size == 4 && ipv6.address.size == 16)
            require(ipv4.address.take(3) == listOf(10.toByte(),77.toByte(),0.toByte()) && (ipv4.address[3].toInt() and 255) in 2..254)
            require(ipv6.address.take(14) == byteArrayOf(0xfd.toByte(),0x77,0,0,0,0,0,0,0,0,0,0,0,0).toList() && ((ipv6.address[14].toInt() and 255) * 256 + (ipv6.address[15].toInt() and 255)) >= 2)
            val ca = p.getString("ca_pem"); require(ca.length <= 32768)
            val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply { load(null) }
            val certs = CertificateFactory.getInstance("X.509").generateCertificates(ca.byteInputStream())
            require(certs.isNotEmpty())
            certs.forEachIndexed { i, cert -> store.setCertificateEntry("ca" + i, cert) }
            val tm = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(store) }
            val ctx = SSLContext.getInstance("TLSv1.3").apply { init(null, tm.trustManagers, null) }
            val tcp = Socket()
            socket = tcp
            check(protect(tcp)) // Exclude tunnel socket before any VPN routes are installed.
            tcp.connect(InetSocketAddress(InetAddress.getByName(host), port), 15000)
            val name = p.getString("server_name")
            val tls = ctx.socketFactory.createSocket(tcp, name, port, true) as SSLSocket
            socket = tls
            tls.enabledProtocols = arrayOf("TLSv1.3")
            tls.sslParameters = tls.sslParameters.apply { endpointIdentificationAlgorithm = "HTTPS"; applicationProtocols = arrayOf("routeloom/1") }
            tls.soTimeout = 15000
            tls.startHandshake()
            check(tls.applicationProtocol == "routeloom/1")
            output = DataOutputStream(tls.outputStream)
            val input = DataInputStream(tls.inputStream)
            frame(1, JSONObject().put("version",1).put("device",device).put("token",token).toString().toByteArray(Charsets.UTF_8))
            val (kind, data) = read(input)
            val welcome = JSONObject(String(data, Charsets.UTF_8))
            check(kind == 2 && welcome.getInt("version") == 1 && welcome.getInt("mtu") == 1280)
            check(InetAddress.getByName(welcome.getString("ipv4")) == ipv4 && InetAddress.getByName(welcome.getString("ipv6")) == ipv6)
            val builder = Builder().setSession("RouteLoom").setMtu(1280).setBlocking(true).setMetered(false)
                .addAddress(ipv4, 24).addAddress(ipv6, 64)
            val routes = p.getJSONArray("routes"); require(routes.length() in 1..64)
            for (i in 0 until routes.length()) {
                val route = routes.getString(i).split('/')
                require(route.size == 2); builder.addRoute(InetAddress.getByName(route[0]), route[1].toInt())
            }
            val dns = p.getJSONArray("dns"); require(dns.length() in 1..4)
            for (i in 0 until dns.length()) builder.addDnsServer(InetAddress.getByName(dns.getString(i)))
            check(!closed.get())
            val fd = builder.establish() ?: error("VPN permission revoked")
            tun = fd
            if (closed.get()) { fd.close(); return }
            tls.soTimeout = 70000
            state = "connected"
            heartbeat.scheduleAtFixedRate({ try { frame(4) } catch (_: Exception) { fail() } }, 20, 20, TimeUnit.SECONDS)
            workers.execute {
                try {
                    val source = FileInputStream(fd.fileDescriptor)
                    val poll = StructPollfd().apply { this.fd = fd.fileDescriptor; events = OsConstants.POLLIN.toShort() }
                    val buffer = ByteArray(1281)
                    while (!closed.get()) {
                        if (Os.poll(arrayOf(poll),1000) == 0) continue
                        val n = source.read(buffer)
                        if (n <= 0) error("TUN closed")
                        val packet = buffer.copyOf(n)
                        try { checkPacket(packet, ipv4.address, ipv6.address, true) } catch (_: IllegalArgumentException) { continue }
                        frame(3, packet)
                    }
                } catch (_: Exception) { fail() }
            }
            val sink = FileOutputStream(fd.fileDescriptor)
            while (!closed.get()) {
                val (type, packet) = read(input)
                when (type) {
                    3 -> { checkPacket(packet,ipv4.address,ipv6.address,false); sink.write(packet) }
                    5 -> check(packet.isEmpty())
                    else -> error("Unexpected frame")
                }
            }
        }
        private fun fail() {
            if (closed.get()) return
            close()
            android.os.Handler(mainLooper).post {
                if (session === this) { session = null; state = "failed"; stopForeground(STOP_FOREGROUND_REMOVE); stopSelf() }
            }
        }
        fun close() {
            if (!closed.compareAndSet(false,true)) return
            try { socket?.close() } catch (_: Exception) {}
            try { tun?.close() } catch (_: Exception) {}
            heartbeat.shutdownNow(); workers.shutdownNow()
        }
    }
}
internal fun checkPacket(p: ByteArray, v4: ByteArray, v6: ByteArray, source: Boolean) {
    require(p.size in 20..1280)
    fun u(i: Int) = p[i].toInt() and 255
    val version = u(0) shr 4
    val offset: Int; val address: ByteArray
    if (version == 4) {
        require((u(0) and 15) * 4 in 20..p.size && (u(2) * 256 + u(3)) == p.size)
        offset = if (source) 12 else 16; address = v4
    } else {
        require(version == 6 && p.size >= 40 && (u(4) * 256 + u(5) + 40) == p.size)
        offset = if (source) 8 else 24; address = v6
    }
    require(p.copyOfRange(offset, offset + address.size).contentEquals(address))
}
