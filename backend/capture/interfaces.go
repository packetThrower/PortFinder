package capture

import (
	"fmt"
	"net"
	"runtime"
	"strings"

	"github.com/google/gopacket/pcap"
)

func ListInterfaces() ([]InterfaceInfo, error) {
	devs, err := pcap.FindAllDevs()
	if err != nil {
		return nil, fmt.Errorf("failed to list interfaces: %w", err)
	}

	interfaces := []InterfaceInfo{
		{Name: "", Description: "Sniff all Interfaces"},
	}

	for _, dev := range devs {
		if isLoopback(dev) {
			continue
		}
		if runtime.GOOS == "windows" && isBluetoothAdapter(dev) {
			continue
		}

		addrs := formatAddresses(dev.Addresses)
		description := dev.Description
		if description == "" {
			description = dev.Name
		}

		interfaces = append(interfaces, InterfaceInfo{
			Name:        dev.Name,
			Description: description,
			Addresses:   addrs,
			HasIP:       hasRoutableIP(dev.Addresses),
		})
	}

	return interfaces, nil
}

func isLoopback(dev pcap.Interface) bool {
	for _, addr := range dev.Addresses {
		if addr.IP.IsLoopback() {
			return true
		}
	}
	return false
}

func isBluetoothAdapter(dev pcap.Interface) bool {
	name := strings.ToLower(dev.Name)
	desc := strings.ToLower(dev.Description)
	return strings.Contains(name, "bluetooth") || strings.Contains(desc, "bluetooth")
}

// hasRoutableIP reports whether any of the interface's addresses is a
// non-loopback, non-link-local IP — i.e. something usable for capture.
func hasRoutableIP(addrs []pcap.InterfaceAddress) bool {
	for _, addr := range addrs {
		ip := addr.IP
		if ip == nil || ip.IsLoopback() || ip.IsLinkLocalUnicast() {
			continue
		}
		return true
	}
	return false
}

func formatAddresses(addrs []pcap.InterfaceAddress) string {
	var parts []string
	for _, addr := range addrs {
		ip := addr.IP
		if ip.To4() != nil || ip.To16() != nil {
			if !ip.Equal(net.IPv6loopback) && !ip.IsLoopback() {
				parts = append(parts, ip.String())
			}
		}
	}
	return strings.Join(parts, ", ")
}
