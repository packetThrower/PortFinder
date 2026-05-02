use crate::CaptureResult;

// LLDP TLV types (7 bits)
const TLV_END: u8 = 0;
const TLV_PORT_ID: u8 = 2;
const TLV_PORT_DESCRIPTION: u8 = 4;
const TLV_SYSTEM_NAME: u8 = 5;
const TLV_SYSTEM_DESCRIPTION: u8 = 6;
const TLV_MGMT_ADDRESS: u8 = 8;
const TLV_ORG_SPECIFIC: u8 = 127;

// IEEE 802.1 OUI for Port VLAN ID TLV
const IEEE_8021_OUI: [u8; 3] = [0x00, 0x80, 0xC2];
const SUBTYPE_PORT_VLAN_ID: u8 = 1;

// IEEE 802.3 OUI for Maximum Frame Size TLV
const IEEE_8023_OUI: [u8; 3] = [0x00, 0x12, 0x0F];
const SUBTYPE_MAX_FRAME_SIZE: u8 = 4;

// TIA / LLDP-MED OUI (TIA-1057). Used by Cisco / Aruba / Avaya / Polycom
// switches and IP phones to advertise voice VLAN, location, and inventory.
const TIA_OUI: [u8; 3] = [0x00, 0x12, 0xBB];
const SUBTYPE_NETWORK_POLICY: u8 = 2;
const APPLICATION_TYPE_VOICE: u8 = 1;

pub fn parse(frame: &[u8]) -> Result<CaptureResult, String> {
    let payload = strip_ethernet_8021q(frame, 0x88CC)?;

    let mut result = CaptureResult {
        switch_name: "N/A".into(),
        switch_ip: "N/A".into(),
        switch_port: "N/A".into(),
        native_vlan: "N/A".into(),
        voice_vlan: "N/A".into(),
        mtu: "N/A".into(),
        switch_model: "N/A".into(),
    };

    let mut port_id_text: Option<String> = None;
    let mut port_desc_text: Option<String> = None;

    for (typ, value) in TlvIter::new(payload) {
        if typ == TLV_END {
            break;
        }
        match typ {
            TLV_PORT_ID
                // value: subtype(1) + id(N). Show id as text.
                if value.len() > 1 => {
                    port_id_text = Some(super::decode_string("lldp/port-id", &value[1..]));
                }
            TLV_PORT_DESCRIPTION => {
                port_desc_text = Some(super::decode_string("lldp/port-description", value));
            }
            TLV_SYSTEM_NAME => {
                result.switch_name = super::decode_string("lldp/system-name", value);
            }
            TLV_SYSTEM_DESCRIPTION => {
                result.switch_model = super::decode_string("lldp/system-description", value);
            }
            TLV_MGMT_ADDRESS => {
                if let Some(ip) = parse_mgmt_address(value) {
                    if result.switch_ip == "N/A" {
                        result.switch_ip = ip;
                    }
                }
            }
            TLV_ORG_SPECIFIC if value.len() >= 4 => {
                let oui = [value[0], value[1], value[2]];
                let subtype = value[3];
                let info = &value[4..];
                if oui == IEEE_8021_OUI && subtype == SUBTYPE_PORT_VLAN_ID && info.len() >= 2 {
                    let vlan = u16::from_be_bytes([info[0], info[1]]);
                    result.native_vlan = vlan.to_string();
                } else if oui == IEEE_8023_OUI
                    && subtype == SUBTYPE_MAX_FRAME_SIZE
                    && info.len() >= 2
                {
                    let mtu = u16::from_be_bytes([info[0], info[1]]);
                    result.mtu = mtu.to_string();
                } else if oui == TIA_OUI && subtype == SUBTYPE_NETWORK_POLICY {
                    if let Some(vlan) = parse_med_voice_vlan(info) {
                        result.voice_vlan = vlan.to_string();
                    }
                }
            }
            _ => {}
        }
    }

    // Combine Port ID and Port Description when both are present, e.g.
    //   "1/1/1 (Duty PC)"
    // Fall back to whichever single value we have, or "N/A".
    result.switch_port = match (port_id_text, port_desc_text) {
        (Some(id), Some(desc)) if id != desc => format!("{id} ({desc})"),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => "N/A".into(),
    };

    Ok(result)
}

fn strip_ethernet_8021q(frame: &[u8], expected_etype: u16) -> Result<&[u8], String> {
    let mut offset = 12;
    if frame.len() < offset + 2 {
        return Err("frame too short for ethernet header".into());
    }
    if frame[offset] == 0x81 && frame[offset + 1] == 0x00 {
        offset += 4;
    }
    if frame.len() < offset + 2 {
        return Err("frame missing ethertype".into());
    }
    let etype = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
    if etype != expected_etype {
        return Err(format!(
            "unexpected ethertype 0x{:04x} (expected 0x{:04x})",
            etype, expected_etype
        ));
    }
    Ok(&frame[offset + 2..])
}

fn parse_mgmt_address(value: &[u8]) -> Option<String> {
    // LLDP Mgmt Address TLV value layout:
    //   addr_string_length(1)  -- includes subtype byte
    //   subtype(1) (IANA address family: 1=IPv4, 2=IPv6)
    //   address(addr_string_length - 1)
    //   ...remainder we ignore
    if value.is_empty() {
        return None;
    }
    let addr_str_len = value[0] as usize;
    if addr_str_len == 0 || value.len() < 1 + addr_str_len {
        return None;
    }
    let subtype = value[1];
    let addr = &value[2..1 + addr_str_len];

    match subtype {
        1 if addr.len() == 4 => {
            Some(std::net::Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]).to_string())
        }
        2 if addr.len() == 16 => {
            let arr: [u8; 16] = addr.try_into().ok()?;
            Some(std::net::Ipv6Addr::from(arr).to_string())
        }
        _ => None,
    }
}

/// Parse the LLDP-MED Network Policy TLV value (TIA-1057). Returns the
/// 12-bit VLAN ID when the policy describes the **Voice** application;
/// other application types (Voice Signaling, Streaming Video, Guest Voice,
/// etc.) are ignored. Layout:
///
///   byte 0       : Application Type
///   bytes 1..4   : packed { U(1) | T(1) | X(1) | VLAN(12) | L2 Prio(3) | DSCP(6) }
///
/// Returns None when the policy is marked Unknown (U bit set), the app
/// type isn't Voice, or the VLAN ID is the reserved 0.
fn parse_med_voice_vlan(info: &[u8]) -> Option<u16> {
    if info.len() < 4 || info[0] != APPLICATION_TYPE_VOICE {
        return None;
    }
    let upper = u16::from_be_bytes([info[1], info[2]]);
    if upper & 0x8000 != 0 {
        // U bit set: the device is signalling "I don't know the policy".
        return None;
    }
    let vlan = (upper >> 1) & 0x0FFF;
    if vlan == 0 {
        // VLAN 0 is reserved; treat as unset.
        return None;
    }
    Some(vlan)
}

/// Iterator over (type, value) pairs in an LLDP TLV stream.
/// Each TLV is: 7-bit type + 9-bit length + value.
struct TlvIter<'a> {
    rest: &'a [u8],
}

impl<'a> TlvIter<'a> {
    fn new(rest: &'a [u8]) -> Self {
        Self { rest }
    }
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = (u8, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.len() < 2 {
            return None;
        }
        let header = u16::from_be_bytes([self.rest[0], self.rest[1]]);
        let typ = (header >> 9) as u8;
        let len = (header & 0x01FF) as usize;
        if 2 + len > self.rest.len() {
            return None;
        }
        let value = &self.rest[2..2 + len];
        self.rest = &self.rest[2 + len..];
        Some((typ, value))
    }
}
