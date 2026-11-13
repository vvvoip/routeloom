use routeloom_tunnel::{invalid, Result};
use socket2::{Domain, Protocol, Socket, Type};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::{TcpStream, UdpSocket};
pub type Protector = Arc<dyn Fn(&Socket, SocketAddr) -> Result<()> + Send + Sync>;
pub fn desktop_protector(endpoint: SocketAddr) -> Protector {
    Arc::new(move |socket, destination| {
        #[cfg(target_os = "linux")]
        {
            socket.set_mark(0x524c)?;
            if destination.ip().is_loopback() {
                return Ok(());
            }
            let family = if endpoint.is_ipv4() { "-4" } else { "-6" };
            let output = std::process::Command::new("ip")
                .args([family, "route", "get", &endpoint.ip().to_string()])
                .output()?;
            let text = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<_> = text.split_whitespace().collect();
            let i = parts
                .iter()
                .position(|v| *v == "dev")
                .ok_or_else(|| invalid("no physical route"))?;
            let name = parts
                .get(i + 1)
                .ok_or_else(|| invalid("no physical interface"))?;
            if name.starts_with("rl") && name[2..].bytes().all(|c| c.is_ascii_digit()) {
                return Err(invalid("physical interface required"));
            }
            socket.bind_device(Some(name.as_bytes()))?;
        }
        #[cfg(target_os = "windows")]
        unsafe {
            use std::os::windows::io::AsRawSocket;
            use windows_sys::Win32::{
                NetworkManagement::IpHelper::GetBestInterfaceEx,
                Networking::WinSock::{
                    setsockopt, IPPROTO_IP, IPPROTO_IPV6, IPV6_UNICAST_IF, IP_UNICAST_IF,
                },
            };
            if destination.ip().is_loopback() {
                return Ok(());
            }
            let address = socket2::SockAddr::from(endpoint);
            let mut index = 0;
            if GetBestInterfaceEx(address.as_ptr().cast(), &mut index) != 0 {
                return Err(invalid("no physical route"));
            }
            let v4 = socket.local_addr()?.is_ipv4();
            let value = if v4 { index.to_be() } else { index };
            if setsockopt(
                socket.as_raw_socket() as usize,
                if v4 { IPPROTO_IP } else { IPPROTO_IPV6 },
                if v4 { IP_UNICAST_IF } else { IPV6_UNICAST_IF },
                (&value as *const u32).cast(),
                4,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (socket, endpoint, destination);
        }
        Ok(())
    })
}
pub async fn tcp(address: SocketAddr, protect: &Protector) -> Result<TcpStream> {
    let s = Socket::new(
        Domain::for_address(address),
        Type::STREAM,
        Some(Protocol::TCP),
    )?;
    s.set_nonblocking(true)?;
    protect(&s, address)?;
    if let Err(e) = s.connect(&address.into()) {
        if e.kind() != std::io::ErrorKind::WouldBlock
            && !matches!(e.raw_os_error(), Some(115 | 36 | 10035))
        {
            return Err(e);
        }
    }
    let stream = TcpStream::from_std(s.into())?;
    tokio::time::timeout(Duration::from_secs(15), stream.writable())
        .await
        .map_err(|_| invalid("connect timeout"))??;
    if let Some(e) = stream.take_error()? {
        return Err(e);
    }
    stream.set_nodelay(true)?;
    Ok(stream)
}
pub async fn udp(address: SocketAddr, protect: &Protector) -> Result<UdpSocket> {
    let s = Socket::new(
        Domain::for_address(address),
        Type::DGRAM,
        Some(Protocol::UDP),
    )?;
    s.set_nonblocking(true)?;
    protect(&s, address)?;
    let bind: SocketAddr = if address.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    }
    .parse()
    .unwrap();
    s.bind(&bind.into())?;
    let socket = UdpSocket::from_std(s.into())?;
    socket.connect(address).await?;
    Ok(socket)
}
