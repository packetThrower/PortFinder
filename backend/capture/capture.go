package capture

import (
	"context"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/google/gopacket"
	"github.com/google/gopacket/pcap"
)

const (
	cdpFilter  = "ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000"
	lldpFilter = "ether proto 0x88cc"
	snapLen    = 65535
	// Short pcap timeout so reads don't block forever.
	// On Linux, BlockForever causes handle.Close() to hang
	// when a read is blocked in another goroutine.
	pcapTimeout = 500 * time.Millisecond
)

func Run(ctx context.Context, req CaptureRequest) (*CaptureResult, error) {
	filter, err := bpfFilter(req.Protocol)
	if err != nil {
		return nil, err
	}

	var packet gopacket.Packet
	if req.InterfaceName == "" {
		packet, err = captureAllInterfaces(ctx, filter)
	} else {
		packet, err = captureInterface(ctx, req.InterfaceName, filter)
	}
	if err != nil {
		return nil, err
	}

	protocol := strings.ToUpper(req.Protocol)
	switch protocol {
	case "CDP":
		return parseCDP(packet)
	case "LLDP":
		return parseLLDP(packet)
	default:
		return nil, fmt.Errorf("unsupported protocol: %s", req.Protocol)
	}
}

func bpfFilter(protocol string) (string, error) {
	switch strings.ToUpper(protocol) {
	case "CDP":
		return cdpFilter, nil
	case "LLDP":
		return lldpFilter, nil
	default:
		return "", fmt.Errorf("unsupported protocol: %s", protocol)
	}
}

func captureInterface(ctx context.Context, ifaceName, filter string) (gopacket.Packet, error) {
	handle, err := pcap.OpenLive(ifaceName, snapLen, true, pcapTimeout)
	if err != nil {
		return nil, fmt.Errorf("failed to open interface %s: %w", ifaceName, err)
	}
	defer handle.Close()

	if err := handle.SetBPFFilter(filter); err != nil {
		return nil, fmt.Errorf("failed to set BPF filter: %w", err)
	}

	// Read packets directly with timeout instead of using Packets() channel.
	// The short pcapTimeout ensures ReadPacketData returns periodically
	// so we can check for context cancellation.
	for {
		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("capture cancelled")
		default:
		}

		data, ci, err := handle.ReadPacketData()
		if err != nil {
			// Timeout — no packet yet, loop and check context
			if err == pcap.NextErrorTimeoutExpired {
				continue
			}
			return nil, fmt.Errorf("read error: %w", err)
		}

		packet := gopacket.NewPacket(data, handle.LinkType(), gopacket.Default)
		packet.Metadata().CaptureInfo = ci
		return packet, nil
	}
}

func captureAllInterfaces(ctx context.Context, filter string) (gopacket.Packet, error) {
	devs, err := pcap.FindAllDevs()
	if err != nil {
		return nil, fmt.Errorf("failed to list interfaces: %w", err)
	}

	resultCh := make(chan gopacket.Packet, 1)
	errCh := make(chan error, 1)

	captureCtx, captureCancel := context.WithCancel(ctx)
	defer captureCancel()

	var wg sync.WaitGroup
	var once sync.Once

	for _, dev := range devs {
		if isLoopback(dev) {
			continue
		}

		wg.Add(1)
		go func(ifaceName string) {
			defer wg.Done()
			pkt, err := captureInterface(captureCtx, ifaceName, filter)
			if err != nil {
				return
			}
			once.Do(func() {
				resultCh <- pkt
				captureCancel()
			})
		}(dev.Name)
	}

	go func() {
		wg.Wait()
		once.Do(func() {
			errCh <- fmt.Errorf("no packet captured from any interface")
		})
	}()

	select {
	case pkt := <-resultCh:
		return pkt, nil
	case err := <-errCh:
		return nil, err
	case <-ctx.Done():
		return nil, fmt.Errorf("capture cancelled")
	}
}
