mod cdp;
mod interfaces;
mod lldp;

pub use interfaces::list_interfaces;

use crate::{CaptureRequest, CaptureResult};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const CDP_FILTER: &str = "ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000";
const LLDP_FILTER: &str = "ether proto 0x88cc";
const SNAP_LEN: i32 = 65535;
const PCAP_TIMEOUT_MS: i32 = 500;

#[derive(Clone, Copy)]
enum Protocol {
    Cdp,
    Lldp,
}

impl Protocol {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CDP" => Ok(Self::Cdp),
            "LLDP" => Ok(Self::Lldp),
            other => Err(format!("unsupported protocol: {other}")),
        }
    }

    fn filter(&self) -> &'static str {
        match self {
            Self::Cdp => CDP_FILTER,
            Self::Lldp => LLDP_FILTER,
        }
    }

    fn parse(&self, frame: &[u8]) -> Result<CaptureResult, String> {
        match self {
            Self::Cdp => cdp::parse(frame),
            Self::Lldp => lldp::parse(frame),
        }
    }
}

pub async fn run(req: CaptureRequest, cancel: CancellationToken) -> Result<CaptureResult, String> {
    let protocol = Protocol::from_str(&req.protocol)?;

    let frame = if req.interface_name.is_empty() {
        capture_all_interfaces(protocol, cancel.clone()).await?
    } else {
        capture_one_interface(req.interface_name.clone(), protocol, cancel.clone()).await?
    };

    protocol.parse(&frame)
}

async fn capture_one_interface(
    iface: String,
    protocol: Protocol,
    cancel: CancellationToken,
) -> Result<Vec<u8>, String> {
    let cancel_clone = cancel.clone();
    let blocking =
        tokio::task::spawn_blocking(move || capture_blocking(&iface, protocol, cancel_clone));

    tokio::select! {
        res = blocking => res.map_err(|e| format!("capture task panicked: {e}"))?,
        _ = cancel.cancelled() => Err("capture cancelled".into()),
    }
}

async fn capture_all_interfaces(
    protocol: Protocol,
    cancel: CancellationToken,
) -> Result<Vec<u8>, String> {
    let devs = pcap::Device::list().map_err(|e| format!("failed to list interfaces: {e}"))?;

    let mut set = JoinSet::new();
    for dev in devs {
        if interfaces::is_loopback(&dev) {
            continue;
        }
        let iface = dev.name.clone();
        let task_cancel = cancel.clone();
        set.spawn_blocking(move || capture_blocking(&iface, protocol, task_cancel));
    }

    if set.is_empty() {
        return Err("no usable interfaces".into());
    }

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err("capture cancelled".into()),
            joined = set.join_next() => match joined {
                None => return Err("no packet captured from any interface".into()),
                Some(Err(e)) => return Err(format!("capture task panicked: {e}")),
                Some(Ok(Ok(frame))) => {
                    cancel.cancel(); // wake the other tasks so they exit cleanly
                    return Ok(frame);
                }
                Some(Ok(Err(_))) => continue, // this iface failed; keep waiting on the others
            }
        }
    }
}

/// Blocking capture loop. Polls pcap with a short timeout so we can react
/// to cancellation between reads.
fn capture_blocking(
    iface: &str,
    protocol: Protocol,
    cancel: CancellationToken,
) -> Result<Vec<u8>, String> {
    let mut cap = pcap::Capture::from_device(iface)
        .map_err(|e| format!("device {iface}: {e}"))?
        .promisc(true)
        .snaplen(SNAP_LEN)
        .timeout(PCAP_TIMEOUT_MS)
        .open()
        .map_err(|e| format!("open {iface}: {e}"))?;

    cap.filter(protocol.filter(), true)
        .map_err(|e| format!("filter on {iface}: {e}"))?;

    loop {
        if cancel.is_cancelled() {
            return Err("capture cancelled".into());
        }
        match cap.next_packet() {
            Ok(packet) => return Ok(packet.data.to_vec()),
            Err(pcap::Error::TimeoutExpired) => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(format!("read on {iface}: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_list_interfaces() {
        let ifaces = list_interfaces().expect("list_interfaces should succeed");
        assert!(!ifaces.is_empty());
        assert_eq!(ifaces[0].name, "", "first entry must be 'Sniff all'");
        // Print so `cargo test -- --nocapture` shows the live result.
        for i in &ifaces {
            println!("  {:<20} hasIP={} addrs={}", i.name, i.has_ip, i.addresses);
        }
    }

    #[test]
    fn cdp_parser_smoke() {
        // Minimal valid CDP frame: ethernet header + SNAP + ver/ttl/checksum + Device ID TLV
        let mut frame = vec![0u8; 12]; // dst+src
        frame.extend_from_slice(&[0x00, 0x40]); // length
        frame.extend_from_slice(&[0xAA, 0xAA, 0x03, 0x00, 0x00, 0x0C, 0x20, 0x00]); // SNAP
        frame.extend_from_slice(&[0x02, 0xb4, 0x00, 0x00]); // ver, ttl, checksum
                                                            // TLV: Device ID = "switch1.example.com"
        let val = b"switch1.example.com";
        let len = (4 + val.len()) as u16;
        frame.extend_from_slice(&[0, 1]); // type
        frame.extend_from_slice(&len.to_be_bytes()); // length
        frame.extend_from_slice(val);
        // TLV: Port ID = "Gi1/0/24"
        let val = b"Gi1/0/24";
        let len = (4 + val.len()) as u16;
        frame.extend_from_slice(&[0, 3]);
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(val);
        // TLV: Native VLAN = 100
        frame.extend_from_slice(&[0, 0x0A, 0, 6, 0x00, 0x64]);

        let result = cdp::parse(&frame).expect("CDP parse should succeed");
        assert_eq!(result.switch_name, "switch1.example.com");
        assert_eq!(result.switch_port, "Gi1/0/24");
        assert_eq!(result.native_vlan, "100");
    }

    #[test]
    fn lldp_parser_smoke() {
        // Minimal LLDP frame: ethernet header + ethertype + TLVs
        let mut frame = vec![0u8; 12]; // dst+src
        frame.extend_from_slice(&[0x88, 0xCC]); // ethertype LLDP
                                                // TLV: System Name (type 5, "switch2")
        let val = b"switch2";
        let header = ((5u16) << 9) | (val.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(val);
        // TLV: Port Description (type 4, "Port 24")
        let val = b"Port 24";
        let header = ((4u16) << 9) | (val.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(val);
        // TLV: Org-specific (type 127), OUI 00:80:C2 subtype 1, VLAN 200
        let body = [0x00, 0x80, 0xC2, 0x01, 0x00, 0xC8];
        let header = ((127u16) << 9) | (body.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(&body);
        // End TLV
        frame.extend_from_slice(&[0, 0]);

        let result = lldp::parse(&frame).expect("LLDP parse should succeed");
        assert_eq!(result.switch_name, "switch2");
        assert_eq!(result.switch_port, "Port 24");
        assert_eq!(result.native_vlan, "200");
    }
}
