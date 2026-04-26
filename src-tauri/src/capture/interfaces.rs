use crate::InterfaceInfo;
use std::net::IpAddr;

pub fn list_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    let devs = pcap::Device::list().map_err(|e| format!("failed to list interfaces: {e}"))?;

    let mut interfaces = vec![InterfaceInfo {
        name: String::new(),
        description: "Sniff all Interfaces".into(),
        addresses: String::new(),
        has_ip: false,
    }];

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
