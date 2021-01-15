#! /bin/bash

# ether[12:2]=0x88cc os for LLDP
# ether[20:2] == 0x2000 is for CDP

bold=$(tput setaf 41)
normal=$(tput sgr0)

tcpdump -nn -v -i enxe4b97a866934 -s 1500 -c 1 'ether[12:2]=0x88cc or ether[20:2] == 0x2000' | sed -n "s/.Port-ID (0x03), value length: 21 bytes:/${bold}Port:${normal} /p;s/.Device-ID (0x01), value length: 15 bytes:/\n${bold}Switch:${normal} /p;s/.Address (0x02), value length: 13 bytes:/${bold}Switch IP:${normal} /p;s/.Native VLAN ID (0x0a), value length: 2 bytes:/${bold}Connected VLAN:${normal} /p;s/.ATA-186 VoIP VLAN request (0x0e), value length: 3 bytes: app 1, vlan/${bold}Connected Voice VLAN:${normal} /p"

# lldp
#tcpdump -nn -v -i enxe4b97a866934 -s 1500 -c 1 'ether proto 0x88cc' 

