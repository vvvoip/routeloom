//! An encrypted, append-only DNS lease journal. IDs are never reassigned, including
//! across profile switches or app restarts. A damaged journal fails closed.
use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    digest::{digest, SHA256},
    rand::{SecureRandom, SystemRandom},
};
use routeloom_tunnel::{config::Profile, invalid, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub struct Journal {
    file: File,
    key: LessSafeKey,
    tag: [u8; 16],
    poisoned: bool,
}
impl Journal {
    pub fn open(directory: &Path, profile: &Profile) -> Result<(Self, Vec<(u16, String)>, u16)> {
        std::fs::create_dir_all(directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let path = directory.join("dns-leases-v1");
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(invalid("invalid DNS journal path"));
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.try_lock().map_err(std::io::Error::other)?;
        if file.metadata()?.len() > 20_000_000 {
            return Err(invalid("DNS journal too large"));
        }
        let key = digest(
            &SHA256,
            format!(
                "RouteLoom DNS journal v1\0{}\0{}\0{}",
                profile.device, profile.endpoint, profile.token
            )
            .as_bytes(),
        );
        let mut tag = [0; 16];
        tag.copy_from_slice(&digest(&SHA256, key.as_ref()).as_ref()[..16]);
        let key = LessSafeKey::new(
            UnboundKey::new(&aead::CHACHA20_POLY1305, key.as_ref())
                .map_err(|_| invalid("DNS key"))?,
        );
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let mut at = 0;
        let mut last = 0u16;
        let mut records = Vec::new();
        while at < bytes.len() {
            if bytes.len() - at < 32 {
                return Err(invalid("truncated DNS journal"));
            }
            let header = &bytes[at..at + 32];
            let id = u16::from_be_bytes([header[0], header[1]]);
            let length = u16::from_be_bytes([header[30], header[31]]) as usize;
            if last == 65534
                || id != last + 1
                || !(17..=269).contains(&length)
                || bytes.len() - at < 32 + length
            {
                return Err(invalid("invalid DNS journal"));
            }
            if header[2..18] == tag {
                let mut nonce = [0; 12];
                nonce.copy_from_slice(&header[18..30]);
                let mut data = bytes[at + 32..at + 32 + length].to_vec();
                let plain = key
                    .open_in_place(
                        Nonce::assume_unique_for_key(nonce),
                        Aad::from(&header[..18]),
                        &mut data,
                    )
                    .map_err(|_| invalid("DNS journal authentication failed"))?;
                let host = std::str::from_utf8(plain)
                    .map_err(|_| invalid("invalid cached DNS name"))?
                    .to_string();
                routeloom_core::Flow::new(&host, None, 53, routeloom_core::Network::Udp)
                    .map_err(|_| invalid("invalid cached DNS name"))?;
                records.push((id, host));
                if records.len() > 4096 {
                    return Err(invalid("DNS cache capacity exceeded"));
                }
            }
            last = id;
            at += 32 + length;
        }
        Ok((
            Self {
                file,
                key,
                tag,
                poisoned: false,
            },
            records,
            last,
        ))
    }
    pub fn append(&mut self, id: u16, host: &str) -> Result<()> {
        if self.poisoned {
            return Err(invalid("DNS journal unavailable"));
        }
        let mut nonce = [0; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| invalid("random source failed"))?;
        let mut header = Vec::from(id.to_be_bytes());
        header.extend_from_slice(&self.tag);
        let mut encrypted = host.as_bytes().to_vec();
        self.key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(&header),
                &mut encrypted,
            )
            .map_err(|_| invalid("DNS encryption failed"))?;
        header.extend_from_slice(&nonce);
        header.extend_from_slice(&(encrypted.len() as u16).to_be_bytes());
        header.extend(encrypted);
        self.poisoned = true;
        self.file.write_all(&header)?;
        self.file.sync_data()?;
        self.poisoned = false;
        Ok(())
    }
}
pub fn default_directory() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/lib/routeloom-client/dns"))
    }
    #[cfg(target_os = "windows")]
    {
        Ok(std::env::current_exe()?
            .parent()
            .ok_or_else(|| invalid("no application directory"))?
            .join("routeloom-state"))
    }
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        Ok(PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| invalid("no application container"))?,
        )
        .join("Library/Application Support/RouteLoomDNS"))
    }
    #[cfg(target_os = "android")]
    {
        Err(invalid("Android requires its private files directory"))
    }
}

#[cfg(test)]
mod journal_tests {
    use super::*;
    fn profile(token: &str) -> Profile {
        serde_json::from_value(serde_json::json!({"endpoint":"192.0.2.1:9443","server_name":"test","ca_pem":"test","device":"test","token":token.repeat(64),"ipv4":"10.77.0.2","ipv6":"fd77::2","routes":["0.0.0.0/0"],"dns":["1.1.1.1"]})).unwrap()
    }
    #[test]
    fn encrypted_leases_survive_restart_and_profile_switches() {
        let dir = std::env::temp_dir().join(format!(
            "routeloom-dns-test-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let (mut journal, rows, last) = Journal::open(&dir, &profile("a")).unwrap();
        assert!(rows.is_empty());
        assert_eq!(last, 0);
        journal.append(1, "first.example").unwrap();
        assert!(Journal::open(&dir, &profile("a")).is_err()); // One writer; prevents address reuse races.
        drop(journal);
        let (mut other, rows, last) = Journal::open(&dir, &profile("b")).unwrap();
        assert!(rows.is_empty());
        assert_eq!(last, 1);
        other.append(2, "second.example").unwrap();
        drop(other);
        let (journal, rows, last) = Journal::open(&dir, &profile("a")).unwrap();
        assert_eq!(rows, vec![(1, "first.example".into())]);
        assert_eq!(last, 2);
        drop(journal);
        let bytes = std::fs::read(dir.join("dns-leases-v1")).unwrap();
        assert!(!bytes.windows(7).any(|x| x == b"example"));
        let mut corrupt = bytes;
        corrupt[40] ^= 1;
        std::fs::write(dir.join("dns-leases-v1"), corrupt).unwrap();
        assert!(Journal::open(&dir, &profile("a")).is_err());
        std::fs::remove_file(dir.join("dns-leases-v1")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
