import subprocess
import os

from prettytable import PrettyTable, ALL
from colorama import Fore, Back, Style, init, deinit

# adapters = os.listdir('/sys/class/net/')
# print(adapters)

# tcpdump -nn -v -i enxe4b97a866934 -s 1500 -c 1 'ether[20:2] == 0x2000'
# for adt in adapters:
# if 'en' in adt:
init()
results = []
t = PrettyTable()
t.title = "Results of the Scan"

p = subprocess.Popen(('sudo', 'tcpdump', '-nn', '-v', '-l', '-s 1500',
                      '-c 1',  'ether[20:2] == 0x2000'), stdout=subprocess.PIPE)
for line in iter(p.stdout.readline, b''):
    line = line.decode("utf-8")
    results.append(line)

print("\n\n")

for row in results:
    if "Port-ID (0x03), value length: 21 bytes:" in row:
        first, second, third = row.split(':')
        t.add_row([Fore.CYAN + "Switchport" + Style.RESET_ALL,
                   Fore.GREEN + third.strip() + Style.RESET_ALL])
    elif "Device-ID (0x01), value length: 15 bytes:" in row:
        first, second, third = row.split(':')
        t.add_row([Fore.CYAN + "Switch" + Style.RESET_ALL,
                   Fore.GREEN + third.strip() + Style.RESET_ALL])
    elif "Address (0x02), value length: 13 bytes:" in row:
        first, second, third = row.split(':')
        ipv4, crap, ip = third.split()
        t.add_row([Fore.CYAN + "IP" + Style.RESET_ALL,
                   Fore.GREEN + ip + Style.RESET_ALL])
    elif "Native VLAN ID (0x0a), value length: 2 bytes:" in row:
        first, second, third = row.split(':')
        t.add_row([Fore.CYAN + "VLAN" + Style.RESET_ALL,
                   Fore.GREEN + third.strip() + Style.RESET_ALL])
    elif "ATA-186 VoIP VLAN request (0x0e), value length: 3 bytes: app 1, vlan" in row:
        first, second, third = row.split(':')
        crap, more_crap, somethin, vlan = third.split()
        t.add_row([Fore.CYAN + "Voice VLAN" + Style.RESET_ALL,
                   Fore.GREEN + vlan + Style.RESET_ALL])

t.hrules = ALL
t.header = False
print(t)

deinit()
