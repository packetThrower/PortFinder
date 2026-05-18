mod cdp;
mod interfaces;
mod lldp;
mod mndp;

pub use interfaces::list_interfaces;

use crate::{CaptureRequest, CaptureResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const CDP_FILTER: &str = "ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000";
const LLDP_FILTER: &str = "ether proto 0x88cc";
const MNDP_FILTER: &str = "udp port 5678";
const SNAP_LEN: i32 = 65535;
// Bound on how long `pcap::next_packet` blocks before returning
// `TimeoutExpired`. The capture loop checks the cancellation token
// between blocking calls, so this is also the worst-case Stop-button
// latency. 50 ms keeps cancellation feeling instant (well under the
// ~100 ms threshold the eye picks up as "snappy") while idle CPU stays
// negligible — at most one wake every 50 ms per interface, which is
// what `race_first` is doing in sniff-all mode anyway.
const PCAP_TIMEOUT_MS: i32 = 50;

/// Decode a TLV byte string with `from_utf8_lossy`. When the input
/// contained non-UTF-8 bytes (i.e. lossy substitution actually
/// happened), log a one-line warning to stderr with the raw bytes so
/// flaky firmware can be diagnosed. The returned `String` is identical
/// to what `from_utf8_lossy(...).into_owned()` would have produced —
/// the displayed result is unchanged, the helper just leaves a
/// breadcrumb on the way through.
pub(crate) fn decode_string(label: &str, bytes: &[u8]) -> String {
    let cow = String::from_utf8_lossy(bytes);
    if matches!(cow, std::borrow::Cow::Owned(_)) {
        let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
        log::warn!(
            "{label} contained non-UTF-8 bytes (raw: {}); decoded with substitutions",
            hex.join(" ")
        );
    }
    cow.into_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Protocol {
    Cdp,
    Lldp,
    Mndp,
}

impl Protocol {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CDP" => Ok(Self::Cdp),
            "LLDP" => Ok(Self::Lldp),
            "MNDP" => Ok(Self::Mndp),
            other => Err(format!("unsupported protocol: {other}")),
        }
    }

    fn filter(&self) -> &'static str {
        match self {
            Self::Cdp => CDP_FILTER,
            Self::Lldp => LLDP_FILTER,
            Self::Mndp => MNDP_FILTER,
        }
    }

    fn parse(&self, frame: &[u8]) -> Result<CaptureResult, String> {
        match self {
            Self::Cdp => cdp::parse(frame),
            Self::Lldp => lldp::parse(frame),
            Self::Mndp => mndp::parse(frame),
        }
    }
}

/// A blocking capture function: given an interface name and a cancellation
/// token, returns a captured frame or an error. Production passes
/// `capture_blocking` (which uses pcap); tests pass a synthetic function.
type BlockingCapture =
    Arc<dyn Fn(String, CancellationToken) -> Result<Vec<u8>, String> + Send + Sync>;

/// Returns true when calling into libpcap / Npcap is safe on the current
/// system. On Windows, calling `pcap::*` functions when Npcap isn't
/// installed crashes the process at the moment `wpcap.dll` resolves —
/// even with delay-loading. Other platforms always have libpcap available
/// (Linux: package dep, macOS: in the base system).
pub(crate) fn pcap_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::privilege::get_privilege_status().npcap_installed
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

pub async fn run(req: CaptureRequest, cancel: CancellationToken) -> Result<CaptureResult, String> {
    if !pcap_available() {
        return Err(
            "Npcap is not installed. Download it from https://npcap.com/#download and re-run."
                .into(),
        );
    }
    let protocol = Protocol::from_str(&req.protocol)?;
    let capture: BlockingCapture = Arc::new(move |name, c| capture_blocking(&name, protocol, c));

    let frame = if req.interface_name.is_empty() {
        let names: Vec<String> = pcap::Device::list()
            .map_err(|e| format!("failed to list interfaces: {e}"))?
            .into_iter()
            .filter(|d| !interfaces::is_loopback(d))
            .map(|d| d.name)
            .collect();
        log::debug!(
            "capture::run: protocol={} mode=sniff-all interfaces={:?}",
            req.protocol,
            names
        );
        race_first(names, cancel.clone(), capture).await?
    } else {
        let name = req.interface_name.clone();
        log::debug!(
            "capture::run: protocol={} mode=single interface={}",
            req.protocol,
            name
        );
        capture_one(cancel.clone(), move || capture(name, cancel)).await?
    };

    log::debug!("capture::run: parsing {}-byte frame", frame.len());
    protocol.parse(&frame)
}

/// Single-interface orchestration. Runs `blocking` on a worker thread and
/// races it against the cancellation token: whichever completes first wins.
async fn capture_one<F>(cancel: CancellationToken, blocking: F) -> Result<Vec<u8>, String>
where
    F: FnOnce() -> Result<Vec<u8>, String> + Send + 'static,
{
    let bg = tokio::task::spawn_blocking(blocking);
    tokio::select! {
        res = bg => res.map_err(|e| format!("capture task panicked: {e}"))?,
        _ = cancel.cancelled() => Err("capture cancelled".into()),
    }
}

/// Multi-interface orchestration. Spawns one blocking task per interface
/// and returns the first one that yields a packet, cancelling the rest.
/// Returns "capture cancelled" if the external token fires first, or
/// "no packet captured from any interface" if every task finishes with
/// an error.
async fn race_first(
    iface_names: Vec<String>,
    cancel: CancellationToken,
    capture: BlockingCapture,
) -> Result<Vec<u8>, String> {
    if iface_names.is_empty() {
        return Err("no usable interfaces".into());
    }

    let mut set = JoinSet::new();
    for name in iface_names {
        let task_cancel = cancel.clone();
        let f = capture.clone();
        set.spawn_blocking(move || f(name, task_cancel));
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
    let mut cap = open_capture(iface)?;

    cap.filter(protocol.filter(), true)
        .map_err(|e| format!("filter on {iface}: {e}"))?;

    log::debug!("capture_blocking: opened {iface} for protocol {protocol:?}");
    loop {
        if cancel.is_cancelled() {
            return Err("capture cancelled".into());
        }
        match cap.next_packet() {
            Ok(packet) => {
                log::debug!("capture_blocking: got {}-byte frame on {iface}", packet.data.len());
                return Ok(packet.data.to_vec());
            }
            Err(pcap::Error::TimeoutExpired) => {
                // Most-common pcap return — the per-read timeout
                // expired without a matching packet. `trace` (not
                // `debug`) because at PCAP_TIMEOUT_MS=50 this
                // fires 20 times per second per interface and
                // would drown out everything else even at
                // `debug` level.
                log::trace!("capture_blocking: timeout on {iface}, retrying");
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                log::warn!("capture_blocking: read on {iface} failed: {e}");
                return Err(format!("read on {iface}: {e}"));
            }
        }
    }
}

/// Open a pcap handle for the interface. Tries promiscuous mode first
/// (needed on mirror/SPAN ports to see traffic destined elsewhere) and
/// falls back to non-promiscuous if the interface refuses — Wi-Fi on
/// macOS rejects BIOCPROMISC, but CDP/LLDP arrive on multicast addresses
/// the NIC accepts in normal mode anyway.
fn open_capture(iface: &str) -> Result<pcap::Capture<pcap::Active>, String> {
    let build = || pcap::Capture::from_device(iface).map_err(|e| format!("device {iface}: {e}"));

    let try_open = |promisc: bool| -> Result<pcap::Capture<pcap::Active>, pcap::Error> {
        build()
            .map_err(pcap::Error::PcapError)?
            .promisc(promisc)
            .snaplen(SNAP_LEN)
            .timeout(PCAP_TIMEOUT_MS)
            .open()
    };

    match try_open(true) {
        Ok(cap) => Ok(cap),
        Err(e) => {
            let msg = e.to_string();
            // macOS Wi-Fi (and some other adapters) reject promisc mode.
            if msg.contains("BIOCPROMISC")
                || msg.contains("Operation not supported")
                || msg.contains("not supported on")
            {
                log::debug!(
                    "open_capture: {iface} rejected promiscuous mode ({msg}); \
                     retrying non-promiscuous"
                );
                try_open(false).map_err(|e| format!("open {iface}: {e}"))
            } else {
                Err(format!("open {iface}: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Protocol ---------------------------------------------------

    #[test]
    fn protocol_from_str_accepts_canonical() {
        assert_eq!(Protocol::from_str("CDP").unwrap(), Protocol::Cdp);
        assert_eq!(Protocol::from_str("LLDP").unwrap(), Protocol::Lldp);
        assert_eq!(Protocol::from_str("MNDP").unwrap(), Protocol::Mndp);
    }

    #[test]
    fn protocol_from_str_is_case_insensitive() {
        assert_eq!(Protocol::from_str("cdp").unwrap(), Protocol::Cdp);
        assert_eq!(Protocol::from_str("Lldp").unwrap(), Protocol::Lldp);
        assert_eq!(Protocol::from_str("mndp").unwrap(), Protocol::Mndp);
    }

    #[test]
    fn protocol_from_str_rejects_unknown() {
        assert!(Protocol::from_str("foo").is_err());
        assert!(Protocol::from_str("").is_err());
    }

    // ---- capture_one (single-interface orchestration) ---------------

    #[tokio::test]
    async fn capture_one_returns_packet() {
        let cancel = CancellationToken::new();
        let result = capture_one(cancel, || Ok(b"hello".to_vec())).await;
        assert_eq!(result, Ok(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn capture_one_propagates_error() {
        let cancel = CancellationToken::new();
        let result = capture_one(cancel, || Err("read failed".into())).await;
        assert_eq!(result, Err("read failed".into()));
    }

    #[tokio::test]
    async fn capture_one_external_cancel_wins_over_slow_capture() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let task = tokio::spawn(async move {
            capture_one(cancel_clone, || {
                // Pretend the capture is stuck waiting. We only need this
                // long enough to lose to the cancel; spawn_blocking can't
                // be killed mid-sleep so anything longer just delays the
                // test runtime's shutdown.
                std::thread::sleep(Duration::from_millis(500));
                Ok(b"never".to_vec())
            })
            .await
        });
        // Give the blocking task a moment to start, then cancel externally.
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_millis(200), task)
            .await
            .expect("did not respond to cancel within 200ms")
            .unwrap();
        assert_eq!(result, Err("capture cancelled".into()));
    }

    #[tokio::test]
    async fn capture_one_panic_is_caught() {
        let cancel = CancellationToken::new();
        let result: Result<Vec<u8>, String> = capture_one(cancel, || panic!("boom")).await;
        assert!(matches!(result, Err(msg) if msg.contains("panicked")));
    }

    // ---- race_first (multi-interface orchestration) -----------------

    #[tokio::test]
    async fn race_first_returns_winner() {
        let cancel = CancellationToken::new();
        let capture: BlockingCapture = Arc::new(|name, c| {
            // "a" loses, "b" wins with a fast result, "c" is the slow
            // sibling that observes the cancel and exits cleanly.
            match name.as_str() {
                "a" => {
                    std::thread::sleep(Duration::from_millis(20));
                    Err("a fail".into())
                }
                "b" => {
                    std::thread::sleep(Duration::from_millis(50));
                    Ok(b"from b".to_vec())
                }
                "c" => {
                    while !c.is_cancelled() {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err("c cancelled".into())
                }
                _ => unreachable!(),
            }
        });

        let result = race_first(
            vec!["a".into(), "b".into(), "c".into()],
            cancel.clone(),
            capture,
        )
        .await;
        assert_eq!(result, Ok(b"from b".to_vec()));
        assert!(cancel.is_cancelled(), "race winner should cancel siblings");
    }

    #[tokio::test]
    async fn race_first_returns_no_packet_when_all_fail() {
        let cancel = CancellationToken::new();
        let capture: BlockingCapture = Arc::new(|_name, _cancel| Err("read failed".into()));
        let result = race_first(vec!["a".into(), "b".into()], cancel, capture).await;
        assert_eq!(result, Err("no packet captured from any interface".into()));
    }

    #[tokio::test]
    async fn race_first_external_cancel() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let capture: BlockingCapture = Arc::new(|_name, c| {
            // Spin until the per-task cancel fires.
            while !c.is_cancelled() {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err("cancelled".into())
        });
        let task = tokio::spawn(async move {
            race_first(vec!["a".into(), "b".into()], cancel_clone, capture).await
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("did not respond to cancel within 2s")
            .unwrap();
        assert_eq!(result, Err("capture cancelled".into()));
    }

    #[tokio::test]
    async fn race_first_empty_list() {
        let cancel = CancellationToken::new();
        let capture: BlockingCapture = Arc::new(|_, _| unreachable!());
        let result = race_first(vec![], cancel, capture).await;
        assert_eq!(result, Err("no usable interfaces".into()));
    }

    // ---- decode_string ----------------------------------------------

    #[test]
    fn decode_string_passes_clean_utf8_through() {
        // Byte-identical to from_utf8_lossy on valid input — no
        // substitutions, no warning emitted.
        assert_eq!(
            decode_string("test/clean", b"switch1.example.com"),
            "switch1.example.com"
        );
    }

    #[test]
    fn decode_string_substitutes_invalid_utf8() {
        // 0xFF is never valid UTF-8; from_utf8_lossy substitutes U+FFFD.
        // We can't easily assert the eprintln output without capturing
        // stderr, but we can at least verify the return value matches
        // what a naked from_utf8_lossy would have produced.
        let bytes = b"hello\xFFworld";
        let want = String::from_utf8_lossy(bytes).into_owned();
        assert_eq!(decode_string("test/dirty", bytes), want);
    }

    // ---- Existing parser smoke tests --------------------------------

    #[test]
    fn smoke_list_interfaces() {
        let ifaces = list_interfaces().expect("list_interfaces should succeed");
        assert!(!ifaces.is_empty());
        assert_eq!(ifaces[0].name, "", "first entry must be 'Sniff all'");
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
        // TLV: Org-specific (type 127), OUI 00:12:0F subtype 4 (Max Frame Size), 9000
        let body = [0x00, 0x12, 0x0F, 0x04, 0x23, 0x28];
        let header = ((127u16) << 9) | (body.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(&body);
        // End TLV
        frame.extend_from_slice(&[0, 0]);

        let result = lldp::parse(&frame).expect("LLDP parse should succeed");
        assert_eq!(result.switch_name, "switch2");
        assert_eq!(result.switch_port, "Port 24");
        assert_eq!(result.native_vlan, "200");
        assert_eq!(result.mtu, "9000");
    }

    #[test]
    fn lldp_parses_med_voice_vlan() {
        // Minimal frame: ethernet + ethertype + LLDP-MED Network Policy TLV
        // for Voice on VLAN 200, tagged, priority 5, DSCP 46 (EF).
        let mut frame = vec![0u8; 12];
        frame.extend_from_slice(&[0x88, 0xCC]);

        // org-specific: OUI 00:12:BB, subtype 2 (Network Policy), then
        // application type Voice (1) + packed flags/VLAN/prio/DSCP.
        // VLAN 200 = 0x0C8. T=1, U=0, X=0. L2 Prio = 5, DSCP = 46.
        // packed bits:
        //   byte 0: 0x40  (U=0 T=1 X=0 V11..V8 = 0000)
        //   byte 1: 0xC8  (V7..V0 = 1100 1000) wait that's V7..V1 + Prio0
        //   actually let me just compute:
        //   value (24 bits) =
        //     0b0_1_0_0000_0000_1100_1000_101_101110
        // Hmm safer to encode the policy fields and compute bit-by-bit.
        let app_type = 0x01u8;
        let u_bit: u32 = 0;
        let t_bit: u32 = 1;
        let x_bit: u32 = 0;
        let vlan: u32 = 200;
        let prio: u32 = 5;
        let dscp: u32 = 46;
        let packed: u32 = (u_bit << 23)
            | (t_bit << 22)
            | (x_bit << 21)
            | ((vlan & 0x0FFF) << 9)
            | ((prio & 0x07) << 6)
            | (dscp & 0x3F);
        let policy = [
            ((packed >> 16) & 0xFF) as u8,
            ((packed >> 8) & 0xFF) as u8,
            (packed & 0xFF) as u8,
        ];

        let mut body = vec![0x00, 0x12, 0xBB, 0x02, app_type];
        body.extend_from_slice(&policy);
        let header = ((127u16) << 9) | (body.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(&body);
        // End TLV
        frame.extend_from_slice(&[0, 0]);

        let result = lldp::parse(&frame).expect("LLDP parse should succeed");
        assert_eq!(result.voice_vlan, "200");
    }

    #[test]
    fn mndp_parser_smoke() {
        // Minimal MNDP frame: ethernet + IPv4 + UDP/5678 + 4-byte MNDP
        // header + a handful of TLVs (Identity, Platform, Board, Interface,
        // IPv4 address). Header values are all zeros — we just skip past.
        let mut frame = vec![0u8; 12]; // dst+src
        frame.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4

        // IPv4: ver/IHL=0x45 (no options), DSCP=0, total length filled in
        // below, id=0, flags/frag=0, TTL=255, proto=17 (UDP), checksum=0
        // (we don't validate it), src=10.0.0.1, dst=255.255.255.255.
        let mut ipv4 = vec![0x45u8, 0x00, 0x00, 0x00, 0, 0, 0, 0, 0xFF, 17, 0, 0];
        ipv4.extend_from_slice(&[10, 0, 0, 1]);
        ipv4.extend_from_slice(&[255, 255, 255, 255]);
        // UDP: sport=5678, dport=5678, length filled below, checksum=0
        let mut udp = vec![0x16, 0x2E, 0x16, 0x2E, 0, 0, 0, 0];

        // MNDP body: 4-byte header + TLVs
        let mut body = vec![0x00, 0x00, 0x00, 0x01]; // header (type+seq)
        let push_tlv = |body: &mut Vec<u8>, typ: u16, value: &[u8]| {
            body.extend_from_slice(&typ.to_be_bytes());
            body.extend_from_slice(&(value.len() as u16).to_be_bytes());
            body.extend_from_slice(value);
        };
        push_tlv(&mut body, 5, b"core-rb");
        push_tlv(&mut body, 8, b"MikroTik");
        push_tlv(&mut body, 12, b"RB951G-2HnD");
        push_tlv(&mut body, 16, b"ether1");
        push_tlv(&mut body, 17, &[192, 168, 88, 1]);

        // Backfill UDP length (header + payload).
        let udp_len = (udp.len() + body.len()) as u16;
        udp[4..6].copy_from_slice(&udp_len.to_be_bytes());
        // Backfill IPv4 total length (header + UDP).
        let ipv4_len = (ipv4.len() + udp.len() + body.len()) as u16;
        ipv4[2..4].copy_from_slice(&ipv4_len.to_be_bytes());

        frame.extend_from_slice(&ipv4);
        frame.extend_from_slice(&udp);
        frame.extend_from_slice(&body);

        let result = mndp::parse(&frame).expect("MNDP parse should succeed");
        assert_eq!(result.switch_name, "core-rb");
        assert_eq!(result.switch_port, "ether1");
        assert_eq!(result.switch_ip, "192.168.88.1");
        assert_eq!(result.switch_model, "MikroTik RB951G-2HnD");
        // VLAN / MTU fields stay unset — MNDP doesn't carry them.
        assert_eq!(result.native_vlan, "N/A");
        assert_eq!(result.voice_vlan, "N/A");
        assert_eq!(result.mtu, "N/A");
    }

    #[test]
    fn lldp_combines_port_id_and_description() {
        // Same minimal LLDP frame as lldp_parser_smoke, but adds a Port
        // ID TLV alongside the Port Description so we can assert the
        // combined "<id> (<desc>)" formatting.
        let mut frame = vec![0u8; 12];
        frame.extend_from_slice(&[0x88, 0xCC]);

        // TLV: Port ID (type 2). value = subtype(1) + id; subtype 5 is
        // "interface name" but we ignore subtype, just emit the id.
        let id = b"1/1/1";
        let body: Vec<u8> = std::iter::once(0x05).chain(id.iter().copied()).collect();
        let header = ((2u16) << 9) | (body.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(&body);

        // TLV: Port Description (type 4)
        let val = b"Duty PC";
        let header = ((4u16) << 9) | (val.len() as u16);
        frame.extend_from_slice(&header.to_be_bytes());
        frame.extend_from_slice(val);

        // End TLV
        frame.extend_from_slice(&[0, 0]);

        let result = lldp::parse(&frame).expect("LLDP parse should succeed");
        assert_eq!(result.switch_port, "1/1/1 (Duty PC)");
    }
}
