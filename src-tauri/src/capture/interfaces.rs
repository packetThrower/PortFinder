use crate::InterfaceInfo;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a successful `list_interfaces` result is reused before we
/// re-scan via libpcap. Short enough that plugging / unplugging a cable
/// shows up on the next refresh-button click after a brief pause; long
/// enough that a focus / blur / focus storm doesn't repeatedly hit
/// `pcap::Device::list` (a few hundred ms on slow hosts).
const CACHE_TTL: Duration = Duration::from_secs(5);

static CACHE: Mutex<Option<(Instant, Vec<InterfaceInfo>)>> = Mutex::new(None);

pub fn list_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    if let Ok(cache) = CACHE.lock() {
        if let Some((stored_at, value)) = cache.as_ref() {
            if stored_at.elapsed() < CACHE_TTL {
                return Ok(value.clone());
            }
        }
    }

    let fresh = list_interfaces_uncached()?;
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((Instant::now(), fresh.clone()));
    }
    Ok(fresh)
}

fn list_interfaces_uncached() -> Result<Vec<InterfaceInfo>, String> {
    let mut interfaces = vec![InterfaceInfo {
        name: String::new(),
        description: "Sniff all Interfaces".into(),
        addresses: String::new(),
        has_ip: false,
    }];

    // On Windows without Npcap installed, calling pcap::Device::list would
    // load wpcap.dll and crash the process. Skip enumeration in that case
    // and return just the placeholder; the privilege-warning banner in the
    // UI explains the situation and links to the Npcap installer.
    if !super::pcap_available() {
        return Ok(interfaces);
    }

    let devs = pcap::Device::list().map_err(|e| format!("failed to list interfaces: {e}"))?;

    for dev in devs {
        if is_loopback(&dev) {
            continue;
        }
        if cfg!(target_os = "windows") && is_bluetooth_adapter(&dev) {
            continue;
        }

        let description = dev
            .desc
            .clone()
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| dev.name.clone());

        interfaces.push(InterfaceInfo {
            name: dev.name.clone(),
            description,
            addresses: format_addresses(&dev.addresses),
            has_ip: has_routable_ip(&dev.addresses),
        });
    }

    Ok(interfaces)
}

pub(crate) fn is_loopback(dev: &pcap::Device) -> bool {
    dev.flags.is_loopback() || dev.addresses.iter().any(|a| a.addr.is_loopback())
}

fn is_bluetooth_adapter(dev: &pcap::Device) -> bool {
    let name = dev.name.to_lowercase();
    let desc = dev.desc.as_deref().unwrap_or("").to_lowercase();
    name.contains("bluetooth") || desc.contains("bluetooth")
}

fn has_routable_ip(addrs: &[pcap::Address]) -> bool {
    addrs.iter().any(|a| match a.addr {
        IpAddr::V4(ip) => !ip.is_loopback() && !ip.is_link_local(),
        IpAddr::V6(ip) => !ip.is_loopback() && !is_ipv6_link_local(&ip),
    })
}

fn format_addresses(addrs: &[pcap::Address]) -> String {
    addrs
        .iter()
        .filter_map(|a| match a.addr {
            IpAddr::V4(ip) if !ip.is_loopback() => Some(ip.to_string()),
            IpAddr::V6(ip) if !ip.is_loopback() => Some(ip.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_ipv6_link_local(ip: &std::net::Ipv6Addr) -> bool {
    // fe80::/10
    let segs = ip.segments();
    (segs[0] & 0xffc0) == 0xfe80
}
