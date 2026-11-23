//! One ICMP echo per call, on whichever mechanism the platform allows without
//! elevation: `IcmpSendEcho` on Windows, an ICMP datagram socket (falling back
//! to raw) on macOS and Linux.

use std::io;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default)]
pub struct Reply {
    pub latency_ms: Option<f32>,
    pub ttl: Option<u8>,
}

/// Resolve a host name or literal address. Windows only speaks ICMPv4 here,
/// so IPv4 is preferred everywhere for consistency.
pub async fn resolve(host: &str) -> io::Result<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip);
    }
    let addrs: Vec<_> = tokio::net::lookup_host((host, 0)).await?.map(|a| a.ip()).collect();
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or(addrs.first())
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no address"))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IP_OPTION_INFORMATION, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
    };

    pub struct Pinger {
        // HANDLE is a raw pointer and not Send; the ICMP handle is safe to use
        // from any thread, one call at a time, which the worker guarantees.
        handle: usize,
        addr: u32,
    }

    impl Pinger {
        pub fn new(ip: IpAddr) -> io::Result<Self> {
            let IpAddr::V4(v4) = ip else {
                return Err(io::Error::new(io::ErrorKind::Unsupported, "IPv6 is not supported on Windows"));
            };
            let h = unsafe { IcmpCreateFile() }.map_err(io::Error::other)?;
            Ok(Self { handle: h.0 as usize, addr: u32::from_ne_bytes(v4.octets()) })
        }

        pub async fn ping(&mut self, ttl: u8, timeout: Duration) -> Reply {
            let (handle, addr) = (self.handle, self.addr);
            let timeout_ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
            tokio::task::spawn_blocking(move || {
                let payload = [0x50u8; 32];
                let opts = IP_OPTION_INFORMATION { Ttl: ttl, ..Default::default() };
                // Reply struct, echoed payload, plus 8 bytes for an ICMP error.
                let mut buf = vec![0u8; size_of::<ICMP_ECHO_REPLY>() + payload.len() + 8];
                let n = unsafe {
                    IcmpSendEcho(
                        HANDLE(handle as *mut _),
                        addr,
                        payload.as_ptr().cast(),
                        payload.len() as u16,
                        Some(&opts),
                        buf.as_mut_ptr().cast(),
                        buf.len() as u32,
                        timeout_ms,
                    )
                };
                if n == 0 {
                    return Reply::default();
                }
                let r = unsafe { std::ptr::read_unaligned(buf.as_ptr().cast::<ICMP_ECHO_REPLY>()) };
                // IP_SUCCESS is 0; anything else is unreachable, TTL expired, etc.
                if r.Status != 0 {
                    return Reply::default();
                }
                Reply { latency_ms: Some(r.RoundTripTime as f32), ttl: Some(r.Options.Ttl) }
            })
            .await
            .unwrap_or_default()
        }
    }

    impl Drop for Pinger {
        fn drop(&mut self) {
            let _ = unsafe { IcmpCloseHandle(HANDLE(self.handle as *mut _)) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    use surge_ping::{Client, Config, ICMP, IcmpPacket, PingIdentifier, PingSequence};

    pub struct Pinger {
        ip: IpAddr,
        ttl: u8,
        client: Client,
        ident: u16,
        seq: u16,
    }

    fn client(ip: IpAddr, ttl: u8) -> io::Result<Client> {
        let kind = if ip.is_ipv6() { ICMP::V6 } else { ICMP::V4 };
        // surge-ping tries an unprivileged datagram socket first and falls back
        // to raw, which the Linux packages make possible with cap_net_raw.
        Client::new(&Config::builder().kind(kind).ttl(u32::from(ttl)).build())
    }

    impl Pinger {
        pub fn new(ip: IpAddr) -> io::Result<Self> {
            let ttl = 128;
            Ok(Self {
                ip,
                ttl,
                client: client(ip, ttl)?,
                ident: std::process::id() as u16 ^ (ip.to_string().len() as u16).rotate_left(8),
                seq: 0,
            })
        }

        pub async fn ping(&mut self, ttl: u8, timeout: Duration) -> Reply {
            if ttl != self.ttl {
                match client(self.ip, ttl) {
                    Ok(c) => {
                        self.client = c;
                        self.ttl = ttl;
                    }
                    Err(_) => return Reply::default(),
                }
            }
            self.seq = self.seq.wrapping_add(1);
            let mut p = self.client.pinger(self.ip, PingIdentifier(self.ident)).await;
            p.timeout(timeout);
            match p.ping(PingSequence(self.seq), &[0x50u8; 32]).await {
                Ok((pkt, rtt)) => Reply {
                    latency_ms: Some(rtt.as_secs_f32() * 1000.0),
                    ttl: match pkt {
                        IcmpPacket::V4(p) => p.get_ttl(),
                        IcmpPacket::V6(p) => Some(p.get_max_hop_limit()),
                    },
                },
                Err(_) => Reply::default(),
            }
        }
    }
}

pub use imp::Pinger;
