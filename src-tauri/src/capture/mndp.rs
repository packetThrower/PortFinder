use crate::CaptureResult;

// MNDP TLV types (subset — we only surface fields that map to
// PortFinder's CaptureResult).
const TLV_IDENTITY: u16 = 5; // UTF-8 system name
const TLV_PLATFORM: u16 = 8; // UTF-8 platform string ("MikroTik")
const TLV_BOARD: u16 = 12; // UTF-8 board name (e.g. "RB951G-2HnD")
const TLV_IPV6: u16 = 15; // 16-byte IPv6 address
const TLV_INTERFACE: u16 = 16; // UTF-8 sender-side interface ("ether1")
const TLV_IPV4: u16 = 17; // 4-byte IPv4 address

const UDP_PORT_MNDP: u16 = 5678;
const MNDP_HEADER_LEN: usize = 4; // type(2) + seq(2), skipped

pub fn parse(frame: &[u8]) -> Result<CaptureResult, String> {
    let payload = strip_to_udp_payload(frame)?;
    if payload.len() < MNDP_HEADER_LEN {
        return Err("MNDP payload too short for header".into());
    }
    let body = &payload[MNDP_HEADER_LEN..];

    let mut result = CaptureResult {
        switch_name: "N/A".into(),
        switch_ip: "N/A".into(),
        switch_port: "N/A".into(),
        native_vlan: "N/A".into(),
        voice_vlan: "N/A".into(),
        mtu: "N/A".into(),
        switch_model: "N/A".into(),
    };

    let mut platform: Option<String> = None;
    let mut board: Option<String> = None;

    for (typ, value) in TlvIter::new(body) {
        match typ {
            TLV_IDENTITY => {
                result.switch_name = super::decode_string("mndp/identity", value);
            }
            TLV_INTERFACE => {
                result.switch_port = super::decode_string("mndp/interface", value);
            }
            TLV_PLATFORM => {
                platform = Some(super::decode_string("mndp/platform", value));
            }
            TLV_BOARD => {
                board = Some(super::decode_string("mndp/board", value));
            }
            TLV_IPV4 if value.len() == 4 && result.switch_ip == "N/A" => {
                result.switch_ip =
                    std::net::Ipv4Addr::new(value[0], value[1], value[2], value[3]).to_string();
            }
            TLV_IPV6 if value.len() == 16 && result.switch_ip == "N/A" => {
                if let Ok(arr) = <[u8; 16]>::try_from(value) {
                    result.switch_ip = std::net::Ipv6Addr::from(arr).to_string();
                }
            }
            _ => {}
        }
    }

    // Combine "MikroTik" + "RB951G-2HnD" → "MikroTik RB951G-2HnD" when both
    // are present, otherwise fall back to whichever single value we have.
    result.switch_model = match (platform, board) {
        (Some(p), Some(b)) if p != b => format!("{p} {b}"),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => "N/A".into(),
    };

    Ok(result)
}

/// Walks past the Ethernet (with optional 802.1Q tag), IPv4, and UDP
/// headers and returns the UDP payload — provided this is a UDP/5678
/// frame. Returns an error otherwise.
fn strip_to_udp_payload(frame: &[u8]) -> Result<&[u8], String> {
    // --- Ethernet --------------------------------------------------------
    let mut offset = 12;
    if frame.len() < offset + 2 {
        return Err("frame too short for ethernet header".into());
    }
    if frame[offset] == 0x81 && frame[offset + 1] == 0x00 {
        offset += 4; // skip 802.1Q tag
    }
    if frame.len() < offset + 2 {
        return Err("frame missing ethertype".into());
    }
    let etype = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
    if etype != 0x0800 {
        return Err(format!(
            "unexpected ethertype 0x{etype:04x} (expected 0x0800 IPv4)"
        ));
    }
    offset += 2;

    // --- IPv4 ------------------------------------------------------------
    if frame.len() < offset + 20 {
        return Err("frame too short for IPv4 header".into());
    }
    let ihl = (frame[offset] & 0x0F) as usize * 4;
    if ihl < 20 || frame.len() < offset + ihl {
        return Err("invalid IPv4 header length".into());
    }
    let proto = frame[offset + 9];
    if proto != 17 {
        return Err(format!("unexpected IP protocol {proto} (expected 17 UDP)"));
    }
    offset += ihl;

    // --- UDP -------------------------------------------------------------
    if frame.len() < offset + 8 {
        return Err("frame too short for UDP header".into());
    }
    let sport = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
    let dport = u16::from_be_bytes([frame[offset + 2], frame[offset + 3]]);
    if sport != UDP_PORT_MNDP && dport != UDP_PORT_MNDP {
        return Err("UDP not on port 5678".into());
    }
    offset += 8;

    Ok(&frame[offset..])
}

/// Iterator over (type, value) pairs in an MNDP TLV stream.
/// Each TLV is: 2-byte type (BE) + 2-byte length (BE) + value.
struct TlvIter<'a> {
    rest: &'a [u8],
}

impl<'a> TlvIter<'a> {
    fn new(rest: &'a [u8]) -> Self {
        Self { rest }
    }
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = (u16, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.len() < 4 {
            return None;
        }
        let typ = u16::from_be_bytes([self.rest[0], self.rest[1]]);
        let len = u16::from_be_bytes([self.rest[2], self.rest[3]]) as usize;
        if 4 + len > self.rest.len() {
            return None;
        }
        let value = &self.rest[4..4 + len];
        self.rest = &self.rest[4 + len..];
        Some((typ, value))
    }
}
