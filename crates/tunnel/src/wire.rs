use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub const ALPN: &[u8] = b"routeloom/1";
pub const MAX_FRAME: usize = 4096;
pub const MTU: usize = 1280;
pub const HELLO: u8 = 1;
pub const WELCOME: u8 = 2;
pub const PACKET: u8 = 3;
pub const PING: u8 = 4;
pub const PONG: u8 = 5;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub version: u8,
    pub device: String,
    pub token: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Welcome {
    pub version: u8,
    pub ipv4: IpAddr,
    pub ipv6: IpAddr,
    pub mtu: u16,
}

/// Caller must not cancel this future and then reuse the stream: reads are partial.
pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<(u8, Vec<u8>)> {
    let len = r.read_u32().await? as usize;
    if !(1..=MAX_FRAME).contains(&len) {
        return Err(invalid("invalid frame size"));
    }
    let kind = r.read_u8().await?;
    let mut data = vec![0; len - 1];
    r.read_exact(&mut data).await?;
    match kind {
        HELLO | WELCOME => {}
        PACKET if data.len() <= MTU => {}
        PING | PONG if data.is_empty() => {}
        _ => return Err(invalid("invalid frame kind or payload")),
    }
    Ok((kind, data))
}
pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, kind: u8, data: &[u8]) -> Result<()> {
    if data.len() >= MAX_FRAME {
        return Err(invalid("frame too large"));
    }
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        w.write_u32((data.len() + 1) as u32).await?;
        w.write_u8(kind).await?;
        w.write_all(data).await?;
        w.flush().await
    })
    .await
    .map_err(|_| invalid("frame write timed out"))?
}
pub fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("invalid message"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fragmented_frames_and_boundaries() {
        let (mut a, mut b) = tokio::io::duplex(5);
        let t = tokio::spawn(async move {
            write_frame(&mut a, PACKET, &[7; MTU]).await.unwrap();
        });
        assert_eq!(read_frame(&mut b).await.unwrap(), (PACKET, vec![7; MTU]));
        t.await.unwrap();
        for raw in [
            &[0, 0, 0, 0][..],
            &[0, 0, 16, 1],
            &[0, 0, 0, 1, 9],
            &[0, 0, 0, 2, 4, 1],
        ] {
            assert!(read_frame(&mut &raw[..]).await.is_err());
        }
    }
}
