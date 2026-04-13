package capture

import (
	"encoding/binary"
	"fmt"
	"net"

	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
)

// IEEE 802.1 OUI for port VLAN ID TLV
const ieee8021OUI = layers.IEEEOUI8021

func parseLLDP(packet gopacket.Packet) (*CaptureResult, error) {
	lldpLayer := packet.Layer(layers.LayerTypeLinkLayerDiscovery)
	if lldpLayer == nil {
		return nil, fmt.Errorf("no LLDP layer found in packet")
	}

	lldp, ok := lldpLayer.(*layers.LinkLayerDiscovery)
	if !ok {
		return nil, fmt.Errorf("failed to parse LLDP layer")
	}

	result := &CaptureResult{
		SwitchName:  "N/A",
		SwitchIP:    "N/A",
		SwitchPort:  string(lldp.PortID.ID),
		NativeVLAN:  "N/A",
		VoiceVLAN:   "N/A",
		SwitchModel: "N/A",
	}

	// Parse the info layer for additional TLVs
	infoLayer := packet.Layer(layers.LayerTypeLinkLayerDiscoveryInfo)
	if infoLayer != nil {
		info, ok := infoLayer.(*layers.LinkLayerDiscoveryInfo)
		if !ok {
			return result, nil
		}

		// System Name
		if info.SysName != "" {
			result.SwitchName = info.SysName
		}

		// Port Description (preferred for switch port display)
		if info.PortDescription != "" {
			result.SwitchPort = info.PortDescription
		}

		// Management Address
		if len(info.MgmtAddress.Address) > 0 {
			result.SwitchIP = formatMgmtAddress(info.MgmtAddress)
		}

		// Parse org-specific TLVs for VLAN info
		for _, tlv := range info.OrgTLVs {
			if tlv.OUI == ieee8021OUI && tlv.SubType == 1 {
				// IEEE 802.1 Port VLAN ID
				if len(tlv.Info) >= 2 {
					vlanID := binary.BigEndian.Uint16(tlv.Info[:2])
					result.NativeVLAN = fmt.Sprintf("%d", vlanID)
				}
			}
		}
	}

	return result, nil
}

func formatMgmtAddress(mgmt layers.LLDPMgmtAddress) string {
	switch mgmt.Subtype {
	case layers.IANAAddressFamilyIPV4:
		if len(mgmt.Address) == 4 {
			return net.IP(mgmt.Address).String()
		}
	case layers.IANAAddressFamilyIPV6:
		if len(mgmt.Address) == 16 {
			return net.IP(mgmt.Address).String()
		}
	}
	// Fallback: try to interpret as IPv4
	if len(mgmt.Address) == 4 {
		return net.IP(mgmt.Address).String()
	}
	return fmt.Sprintf("%x", mgmt.Address)
}

