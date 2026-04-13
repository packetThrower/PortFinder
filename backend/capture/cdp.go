package capture

import (
	"fmt"

	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
)

func parseCDP(packet gopacket.Packet) (*CaptureResult, error) {
	cdpLayer := packet.Layer(layers.LayerTypeCiscoDiscoveryInfo)
	if cdpLayer == nil {
		return nil, fmt.Errorf("no CDP layer found in packet")
	}

	cdpInfo, ok := cdpLayer.(*layers.CiscoDiscoveryInfo)
	if !ok {
		return nil, fmt.Errorf("failed to parse CDP layer")
	}

	result := &CaptureResult{
		SwitchName:  cdpInfo.DeviceID,
		SwitchPort:  cdpInfo.PortID,
		SwitchModel: cdpInfo.Platform,
		NativeVLAN:  fmt.Sprintf("%d", cdpInfo.NativeVLAN),
		VoiceVLAN:   "N/A",
		SwitchIP:    "N/A",
	}

	// Extract management IP address
	if len(cdpInfo.MgmtAddresses) > 0 {
		result.SwitchIP = cdpInfo.MgmtAddresses[0].String()
	} else if len(cdpInfo.Addresses) > 0 {
		result.SwitchIP = cdpInfo.Addresses[0].String()
	}

	// Extract Voice VLAN from VLANReply
	if cdpInfo.VLANReply.VLAN != 0 {
		result.VoiceVLAN = fmt.Sprintf("%d", cdpInfo.VLANReply.VLAN)
	}

	return result, nil
}

