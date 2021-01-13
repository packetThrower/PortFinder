import tkinter
import tkinter.ttk
import netifaces
import sys

from tkinter import messagebox

from scapy import sendrecv
from scapy.arch.windows import get_windows_if_list
from scapy.main import load_contrib

# Needs to be imported for pyinstaller
from scapy.contrib import cdp

from platform import system
from pathlib import Path


#######################################################################################################################


def stop_thread():
    A_S.stop()
    progressb.grid_remove()

#######################################################################################################################


def run_scan():
    global A_S
    progressb.grid()
    progressb.start()
    
    # sniff for the CDP packet
    if nic_selector.get() != '':
            A_S = sendrecv.AsyncSniffer(iface=nic_selector.get(), prn=process_packets,
                            store=0, filter="ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000", count=1)
            A_S.start()
    else:
        A_S = sendrecv.AsyncSniffer(prn=process_packets,
                        store=0, filter="ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000", count=1)
        A_S.start()
    
#######################################################################################################################


def process_packets(pkt):
    """
    Function for processing packets and printing information of CDP Packets
    """
    # Clear existing boxes if not clear
    ent_switchport.delete(0, tkinter.END)
    ent_switch.delete(0, tkinter.END)
    ent_ip.delete(0, tkinter.END)
    ent_vlan.delete(0, tkinter.END)
    ent_voicevlan.delete(0, tkinter.END)
    ent_model.delete(0, tkinter.END)

    pkt.show()

    try:
        ent_switchport.insert(0, pkt['CDPMsgPortID'].iface.decode())
        ent_switch.insert(0, pkt['CDPMsgDeviceID'].val.decode())
        ent_ip.insert(0, pkt['CDPMsgMgmtAddr'].addr[0].addr)
        ent_vlan.insert(0, str(pkt['CDPMsgNativeVLAN'].vlan))
        ent_voicevlan.insert(0, str(pkt['CDPMsgVoIPVLANReply'].vlan))
        ent_model.insert(0, pkt['CDPMsgPlatform'].val.decode())
    except Exception as e1:
        what_to_say = e1.args[0] + "\n\nTry rerunning the scan."
        messagebox.showerror(title="Bad Packet", message=what_to_say)

    progressb.grid_remove()


#######################################################################################################################

load_contrib("cdp")

# find the pictures after 
if getattr(sys, 'frozen', False) and hasattr(sys, '_MEIPASS'):
    bundle_dir = Path(sys._MEIPASS)
else:
    bundle_dir = Path(__file__).parent

otecc_png = Path.cwd() / bundle_dir / "otecc.png"
otecC_small_png = Path.cwd() / bundle_dir / "otecc_small.png"

root = tkinter.Tk()
root.title("  Get Port Info")
root.tk.call('wm', 'iconphoto', root._w, tkinter.PhotoImage(file=otecc_png))

content = tkinter.Frame(root, padx=10, pady=10)
btn_frame = tkinter.Frame(content)

# instantiate widgets
lbl_switch = tkinter.Label(content, text="Switch: ", fg="blue", pady=3)
lbl_ip = tkinter.Label(content, text="IP: ", fg="blue", pady=3)
lbl_switchport = tkinter.Label(content, text="Switchport: ", fg="blue", pady=3)
lbl_vlan = tkinter.Label(content, text="VLAN: ", fg="blue", pady=3)
lbl_voicevlan = tkinter.Label(content, text="Voice VLAN: ", fg="blue", pady=3)
lbl_model = tkinter.Label(content, text="Switch Model: ", fg="blue", pady=3)
lbl_spacer = tkinter.Label(content, text="", pady=3)

ent_switch = tkinter.Entry(content, width="30")
ent_ip = tkinter.Entry(content, width="30")
ent_switchport = tkinter.Entry(content, width="30")
ent_vlan = tkinter.Entry(content, width="30")
ent_voicevlan = tkinter.Entry(content, width="30")
ent_model = tkinter.Entry(content, width="30")

lbl_nic_selector = tkinter.Label(
    content, text="Select a NIC: ", padx=10, pady=10)
nic_selector = tkinter.ttk.Combobox(content, width="25")

progressb = tkinter.ttk.Progressbar(
    content, orient=tkinter.HORIZONTAL, length=300, mode='indeterminate')
progressb.start(10)
progressb.step(100)

otec_photo = tkinter.PhotoImage(file=otecC_small_png)
lbl_photo = tkinter.Label(content, image=otec_photo,
                          anchor="e", justify=tkinter.LEFT, width="200")

start_button = tkinter.Button(
    btn_frame, text="Start", state="active", padx=60, pady=5, command=run_scan)

stop_button = tkinter.Button(
    btn_frame, text="Stop", state="active", padx=60, pady=5, command=stop_thread)

# Get all the NICs and put then into a list.
nics=[]
if system() == "Windows":
    windows_nics = get_windows_if_list()
    for interface in windows_nics:
        if "bluetooth" not in interface['name']:
            nics.append(interface['name'])
else:
    nics = netifaces.interfaces()

nic_selector['values'] = nics

# put widgets into the window
content.grid(row=0, column=0)

lbl_nic_selector.grid(row=0, column=0)
nic_selector.grid(row=0, column=1)

lbl_switch.grid(row=1, column=0)
ent_switch.grid(row=1, column=1)

lbl_ip.grid(row=2, column=0)
ent_ip.grid(row=2, column=1)

lbl_switchport.grid(row=3, column=0)
ent_switchport.grid(row=3, column=1)

lbl_vlan.grid(row=4, column=0)
ent_vlan.grid(row=4, column=1)

lbl_voicevlan.grid(row=5, column=0)
ent_voicevlan.grid(row=5, column=1)

lbl_model.grid(row=6, column=0)
ent_model.grid(row=6, column=1)

lbl_spacer.grid(row=7, column=0)
progressb.grid(row=7, column=0, columnspan=2)
progressb.grid_remove()

btn_frame.grid(row=8, column=0, columnspan=2)
start_button.grid(row=0, column=0)
stop_button.grid(row=0, column=1)

lbl_photo.grid(row=9, column=1, pady=3)

root.mainloop()


