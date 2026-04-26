use crate::CaptureResult;

// CDP TLV types (subset we care about)
const TLV_DEVICE_ID: u16 = 0x0001;
const TLV_ADDRESSES: u16 = 0x0002;
const TLV_PORT_ID: u16 = 0x0003;
const TLV_PLATFORM: u16 = 0x0006;
const TLV_NATIVE_VLAN: u16 = 0x000A;
const TLV_VOICE_VLAN: u16 = 0x000E; // VLANReply
const TLV_MGMT_ADDRESS: u16 = 0x0016;

// SNAP header bytes that precede the CDP payload inside an 802.3 frame:
//   AA AA 03 00 00 0C 20 00
// We accept frames with optional 802.1Q VLAN tag (4 extra bytes after src MAC).
const SNAP_LLC: [u8; 8] = [0xAA, 0xAA, 0x03, 0x00, 0x00, 0x0C, 0x20, 0x00];

pub fn parse(frame: &[u8]) -> Result<CaptureResult, String> {
    let payload = strip_ethernet_and_snap(frame)?;
    let tlvs = &payload.get(4..).ok_or("CDP payload truncated")?; // skip ver, ttl, checksum (1+1+2)

    let mut result = CaptureResult {
        switch_name: String::new(),
        switch_ip: "N/A".into(),
        switch_port: String::new(),
        native_vlan: "N/A".into(),
        voice_vlan: "N/A".into(),
        mtu: "N/A".into(),
        switch_model: "N/A".into(),
    };

    for (typ, value) in TlvIter::new(tlvs) {
        match typ {
            TLV_DEVICE_ID => result.switch_name = lossy(value),
            TLV_PORT_ID => result.switch_port = lossy(value),
            TLV_PLATFORM => result.switch_model = lossy(value),
            TLV_NATIVE_VLAN if value.len() >= 2 => {
                result.native_vlan = u16::from_be_bytes([value[0], value[1]]).to_string();
            }
            TLV_VOICE_VLAN
                // Voice VLAN TLV layout: appliance ID (1 byte), VLAN (2 bytes)
                if value.len() >= 3 => {
                    let vlan = u16::from_be_bytes([value[1], value[2]]);
                    if vlan != 0 {
                        result.voice_vlan = vlan.to_string();
                    }
                }
            TLV_MGMT_ADDRESS | TLV_ADDRESSES => {
                if let Some(ip) = first_address_from_tlv(value) {
                    if result.switch_ip == "N/A" {
                        result.switch_ip = ip;
                    }
                }
            }
            _ => {}
        }
    }

    if result.switch_name.is_empty() && result.switch_port.is_empty() {
        return Err("no CDP info found in packet".into());
    }
    Ok(result)
}

fn strip_ethernet_and_snap(frame: &[u8]) -> Result<&[u8], String> {
    // Ethernet: dst(6) + src(6) + (optional VLAN 4) + length/etype(2)
    let mut offset = 12;
    if frame.len() < offset + 2 {
        return Err("frame too short for ethernet header".into());
    }
    // 802.1Q tagged?
    if frame[offset] == 0x81 && frame[offset + 1] == 0x00 {
        offset += 4; // skip VLAN tag (TPID + TCI)
    }
    // Now offset points at length/EtherType field. CDP uses 802.3 length, not EtherType.
    offset += 2;

    let snap_end = offset + SNAP_LLC.len();
    if frame.len() < snap_end {
        return Err("frame too short for SNAP/CDP header".into());
    }
    if frame[offset..snap_end] != SNAP_LLC {
        return Err("frame is not CDP (SNAP header mismatch)".into());
    }
    Ok(&frame[snap_end..])
}

fn first_address_from_tlv(value: &[u8]) -> Option<String> {
    // Address TLV layout:
    //   number_of_addresses (4 bytes)
    //   for each: protocol_type(1) + protocol_length(1) + protocol(P) + address_length(2) + address(N)
    if value.len() < 4 {
        return None;
    }
    let count = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
    if count == 0 {
        return None;
    }
    let mut p = 4;
    if p + 2 > value.len() {
        return None;
    }
    let proto_len = value[p + 1] as usize;
    p += 2 + proto_len;
    if p + 2 > value.len() {
        return None;
    }
    let addr_len = u16::from_be_bytes([value[p], value[p + 1]]) as usize;
    p += 2;
    if p + addr_len > value.len() {
        return None;
    }
    let addr = &value[p..p + addr_len];
    Some(format_ip(addr))
}

fn format_ip(bytes: &[u8]) -> String {
    match bytes.len() {
        4 => std::net::Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]).to_string(),
        16 => {
            let arr: [u8; 16] = bytes.try_into().unwrap();
            std::net::Ipv6Addr::from(arr).to_string()
        }
        _ => bytes.iter().map(|b| format!("{:02x}", b)).collect(),
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Iterator over (type, value) pairs in a CDP TLV stream.
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
        if len < 4 || len > self.rest.len() {
            return None;
        }
        let value = &self.rest[4..len];
        self.rest = &self.rest[len..];
        Some((typ, value))
    }
}
