use routeloom_tunnel::{
    config::{token_hash, Device, Profile},
    tls, wire,
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
fn identity() -> (String, String) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    (key.cert.pem(), key.signing_key.serialize_pem())
}
async fn fixture() -> (Profile, tokio::task::JoinHandle<bool>) {
    let (cert, key) = identity();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let p = Profile {
        endpoint: listener.local_addr().unwrap(),
        server_name: "localhost".into(),
        ca_pem: cert.clone(),
        device: "test".into(),
        token: "ab".repeat(32),
        ipv4: "10.77.0.2".parse().unwrap(),
        ipv6: "fd77::2".parse().unwrap(),
        routes: vec!["0.0.0.0/0".parse().unwrap(), "::/0".parse().unwrap()],
        dns: vec!["1.1.1.1".parse().unwrap()],
    };
    let device = Device {
        id: p.device.clone(),
        token_sha256: token_hash(&p.token),
        ipv4: p.ipv4,
        ipv6: p.ipv6,
    };
    let acceptor = TlsAcceptor::from(tls::server_config(cert.as_bytes(), key.as_bytes()).unwrap());
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let Ok(mut stream) = acceptor.accept(tcp).await else {
            return false;
        };
        assert_eq!(
            stream.get_ref().1.protocol_version(),
            Some(tokio_rustls::rustls::ProtocolVersion::TLSv1_3)
        );
        assert_eq!(stream.get_ref().1.alpn_protocol(), Some(wire::ALPN));
        let Ok((kind, data)) = wire::read_frame(&mut stream).await else {
            return false;
        };
        let hello: wire::Hello = serde_json::from_slice(&data).unwrap();
        if kind != wire::HELLO || !device.authenticates(&hello.device, &hello.token) {
            return false;
        }
        wire::write_frame(
            &mut stream,
            wire::WELCOME,
            &wire::json(&device.welcome()).unwrap(),
        )
        .await
        .unwrap();
        let (kind, packet) = wire::read_frame(&mut stream).await.unwrap();
        assert_eq!(kind, wire::PACKET);
        wire::write_frame(&mut stream, kind, &packet).await.unwrap();
        true
    });
    (p, task)
}
#[tokio::test]
async fn authenticated_encrypted_roundtrip() {
    let (profile, server) = fixture().await;
    let mut stream = tls::connect(&profile).await.unwrap();
    let packet = vec![0x42; 1280];
    wire::write_frame(&mut stream, wire::PACKET, &packet)
        .await
        .unwrap();
    assert_eq!(
        wire::read_frame(&mut stream).await.unwrap(),
        (wire::PACKET, packet)
    );
    assert!(server.await.unwrap());
}
#[tokio::test]
async fn wrong_ca_hostname_token_and_lease_are_rejected() {
    for case in 0..4 {
        let (mut p, server) = fixture().await;
        match case {
            0 => p.ca_pem = identity().0,
            1 => p.server_name = "wrong.example".into(),
            2 => p.token = "cd".repeat(32),
            _ => p.ipv4 = "10.77.0.3".parse().unwrap(),
        }
        assert!(tls::connect(&p).await.is_err());
        // The fixture may be waiting for a packet after sending its real lease.
        server.abort();
    }
}
#[test]
fn empty_and_untrusted_ca_are_not_accepted() {
    assert!(tls::client_config(b"").is_err());
    assert!(tls::server_config(b"bad", b"bad").is_err());
}
