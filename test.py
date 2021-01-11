from scapy.all import *


def process_packets(pkt):
    """
    Function for processing packets and printing information of CDP Packets
    """
    print(pkt.sniffed_on)
    print("Switch: " + pkt['CDPMsgDeviceID'].val.decode())
    print("Switch IP: " + pkt['CDPMsgMgmtAddr'].addr[0].addr)
    print("Switch Port: " + pkt['CDPMsgPortID'].iface.decode())
    print("VLAN: " + str(pkt['CDPMsgNativeVLAN'].vlan))
    print("Voice VLAN: " + str(pkt['CDPMsgVoIPVLANReply'].vlan))
    print("Switch Type: " + pkt['CDPMsgPlatform'].val.decode())


if __name__ == "__main__":
    load_contrib("cdp")

    sniff(iface="enxe4b97a866934", prn=process_packets,
          store=0, filter="ether dst 01:00:0c:cc:cc:cc", count=1)
