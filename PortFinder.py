import tkinter
import tkinter.ttk
import netifaces
import sys

from tkinter import messagebox

from platform import system
from pathlib import Path

from scapy import sendrecv
if system() == "Windows":
    from scapy.arch.windows import get_windows_if_list
from scapy.main import load_contrib

# Needs to be imported for pyinstaller
from scapy.contrib import cdp
from scapy.contrib import lldp


#######################################################################################################################


def hextoip(ipadd):
    """
    convert HEX to IP Dot format
    """
    num = 2
    return '.'.join([str(int(ipadd[i:i+num], 16)) for i in range(0, len(ipadd), num)])


#######################################################################################################################


def stop_thread():
    A_S.stop()
    pfw.progressb.grid_remove()


#######################################################################################################################

def run_scan():
    global A_S
    pfw.progressb.grid()
    pfw.progressb.start()

    cdp_filter = "ether[12:2] <= 1500 && ether[14:2] == 0xAAAA && ether[16:1] == 0x03 && ether[17:2] == 0x0000 && ether[19:1] == 0x0C && ether[20:2] == 0x2000"
    lldp_filter = "ether proto 0x88cc"
    
    # sniff for the CDP packet
    if pfw.r_protocol.get() == "CDP":
        if pfw.nic_selector.get() == "Sniff all Interfaces":
                A_S = sendrecv.AsyncSniffer(prn=process_packets,
                                store=0, filter=cdp_filter, count=1)
                A_S.start()
        else:
            A_S = sendrecv.AsyncSniffer(iface=pfw.nic_selector.get(), prn=process_packets,
                            store=0, filter=cdp_filter, count=1)
            A_S.start()
    # sniff for lldp
    elif pfw.r_protocol.get() == "LLDP":
        if pfw.nic_selector.get() == 'Sniff all Interfaces':
                A_S = sendrecv.AsyncSniffer( prn=process_packets,
                                store=0, filter=lldp_filter, count=1)
                A_S.start()
        else:
            A_S = sendrecv.AsyncSniffer(iface=pfw.nic_selector.get(), prn=process_packets,
                            store=0, filter=lldp_filter, count=1)
            A_S.start()
    
#######################################################################################################################


def process_packets(pkt):
    """
    Function for processing packets and printing information of CDP Packets
    """

    pfw.ent_switchport.configure(state="normal")
    pfw.ent_switch.configure(state="normal")
    pfw.ent_ip.configure(state="normal")
    pfw.ent_vlan.configure(state="normal")
    pfw.ent_voicevlan.configure(state="normal")
    pfw.ent_model.configure(state="normal")

    # Clear existing boxes if not clear
    pfw.ent_switchport.delete(0, tkinter.END)
    pfw.ent_switch.delete(0, tkinter.END)
    pfw.ent_ip.delete(0, tkinter.END)
    pfw.ent_vlan.delete(0, tkinter.END)
    pfw.ent_voicevlan.delete(0, tkinter.END)
    pfw.ent_model.delete(0, tkinter.END)

    # pkt.show()

    if pfw.r_protocol.get() == "CDP":
        try:
            pfw.ent_switchport.insert(0, pkt['CDPMsgPortID'].iface.decode())
            pfw.ent_switch.insert(0, pkt['CDPMsgDeviceID'].val.decode())
            pfw.ent_ip.insert(0, pkt['CDPMsgMgmtAddr'].addr[0].addr)
            pfw.ent_vlan.insert(0, str(pkt['CDPMsgNativeVLAN'].vlan))
            pfw.ent_voicevlan.insert(0, str(pkt['CDPMsgVoIPVLANReply'].vlan))
            pfw.ent_model.insert(0, pkt['CDPMsgPlatform'].val.decode())
        except Exception as e1:
            what_to_say = e1.args[0] + "\n\nTry rerunning the scan."
            messagebox.showerror(title="Bad Packet", message=what_to_say)

    if pfw.r_protocol.get() == "LLDP":
        try:
            pfw.ent_switchport.insert(0, pkt['LLDPDUPortDescription'].description)
            pfw.ent_switch.insert(0, pkt['LLDPDUSystemName'].system_name)
            pfw.ent_ip.insert(0, hextoip(pkt['LLDPDUManagementAddress'].management_address.hex()))
            pfw.ent_vlan.insert(0, int(pkt['LLDPDUGenericOrganisationSpecific'].data.hex()))
            pfw.ent_voicevlan.insert(0, "N/A")
            pfw.ent_model.insert(0, "N/A")
        except Exception as e1:
            what_to_say = e1.args[0] + "\n\nTry rerunning the scan."
            messagebox.showerror(title="Bad Packet", message=what_to_say)

    pfw.ent_switchport.configure(state="readonly")
    pfw.ent_switch.configure(state="readonly")
    pfw.ent_ip.configure(state="readonly")
    pfw.ent_vlan.configure(state="readonly")
    pfw.ent_voicevlan.configure(state="readonly")
    pfw.ent_model.configure(state="readonly")

    pfw.progressb.grid_remove()


#######################################################################################################################


def main():
    load_contrib("cdp")
    load_contrib("lldp")

    global pfw 
    pfw = Port_Finder_Window()
    pfw.start()


#######################################################################################################################

"""
    Class for building the main Window.
"""

class Port_Finder_Window:
    def __init__(self) -> None:
        super().__init__()

        # find the pictures after loading
        if getattr(sys, 'frozen', False) and hasattr(sys, '_MEIPASS'):
            bundle_dir = Path(sys._MEIPASS)
        else:
            bundle_dir = Path(__file__).parent
        self.otecc_png = Path.cwd() / bundle_dir / "otecc.png"
        self.otecc_small_png = Path.cwd() / bundle_dir / "otecc_small.png"

        # setup root window
        self.root = tkinter.Tk()
        self.root.title("  Get Port Info")
        self.root.tk.call('wm', 'iconphoto', self.root._w, tkinter.PhotoImage(file=self.otecc_png))

        # create some containers to hold widgets
        self.content = tkinter.Frame(self.root, padx=10, pady=10, bg="#505050")
        self.btn_frame = tkinter.Frame(self.content, bg="#505050")

        # variable for radio buttons and combobox
        self.r_protocol = tkinter.StringVar(self.content, "CDP")
        self.combo_box_val = tkinter.StringVar(self.content, "Sniff all Interfaces")

        # setup the combobox
        self.lbl_nic_selector = tkinter.Label(
            self.content, text="Select a NIC: ", padx=10, pady=10, bg="#505050", fg="white")
        self.nic_selector = tkinter.ttk.Combobox(self.content, textvariable=self.combo_box_val, state="readonly", width="25")

        # Get all the NICs and put then into a list.
        nics=[]

        if system() == "Windows":
            windows_nics = get_windows_if_list()
            # Eliminate useless NICs
            for interface in windows_nics:
                if "bluetooth" not in interface['name']:
                    nics.append(interface['name'])
                    nics.append("Sniff all Interfaces")
        else:
            nics = netifaces.interfaces()
            nics.append("Sniff all Interfaces")

        self.nic_selector['values'] = nics

        # instantiate widgets
        self.lbl_switch = tkinter.Label(self.content, text="Switch: ", fg="white", pady=3, bg="#505050")
        self.ent_switch = tkinter.Entry(self.content, width="29")

        self.lbl_ip = tkinter.Label(self.content, text="Switch IP: ", fg="white", pady=3, bg="#505050")
        self.ent_ip = tkinter.Entry(self.content, width="29")

        self.lbl_switchport = tkinter.Label(self.content, text="Switchport: ", fg="white", pady=3, bg="#505050")
        self.ent_switchport = tkinter.Entry(self.content, width="29")

        self.lbl_vlan = tkinter.Label(self.content, text="VLAN: ", fg="white", pady=3, bg="#505050")
        self.ent_vlan = tkinter.Entry(self.content, width="29")

        self.lbl_voicevlan = tkinter.Label(self.content, text="Voice VLAN: ", fg="white", pady=3, bg="#505050")
        self.ent_voicevlan = tkinter.Entry(self.content, width="29")

        self.lbl_model = tkinter.Label(self.content, text="Switch Model: ", fg="white", pady=3, bg="#505050")
        self.ent_model = tkinter.Entry(self.content, width="29")

        self.lbl_spacer = tkinter.Label(self.content, text="", pady=3, bg="#505050")

        # setup the progress bar
        self.progressb = tkinter.ttk.Progressbar(self.content, orient=tkinter.HORIZONTAL, length=300, mode='indeterminate')
        self.progressb.start(10)
        self.progressb.step(100)
        self.progressb.grid_remove()

        # setup the start/stop buttons
        self.start_button = tkinter.Button(
            self.btn_frame, text="Start", state="active", padx=60, pady=5, command=run_scan)
        self.stop_button = tkinter.Button(
            self.btn_frame, text="Stop", state="active", padx=60, pady=5, command=stop_thread)

        # add watermark
        self.otec_photo = tkinter.PhotoImage(file=self.otecc_small_png)
        self.lbl_photo = tkinter.Label(self.content, image=self.otec_photo, anchor="e", justify=tkinter.LEFT, width="200", bg="#505050")

        # setup the radio buttons
        self.style = tkinter.ttk.Style(self.content)
        self.style.configure("TRadiobutton", background="#505050", foreground="white")
        self.cdp_radiobtn = tkinter.ttk.Radiobutton(self.content, text="CDP", variable=self.r_protocol, value="CDP")
        self.lldp_radiobtn = tkinter.ttk.Radiobutton(self.content, text="LLDP", variable=self.r_protocol, value="LLDP")

        # put widgets into the window
        self.content.grid(row=0, column=0)

        self.lbl_nic_selector.grid(row=0, column=0)
        self.nic_selector.grid(row=0, column=1)

        self.lbl_switch.grid(row=1, column=0)
        self.ent_switch.grid(row=1, column=1)

        self.lbl_ip.grid(row=2, column=0)
        self.ent_ip.grid(row=2, column=1)

        self.lbl_switchport.grid(row=3, column=0)
        self.ent_switchport.grid(row=3, column=1)

        self.lbl_vlan.grid(row=4, column=0)
        self.ent_vlan.grid(row=4, column=1)

        self.lbl_voicevlan.grid(row=5, column=0)
        self.ent_voicevlan.grid(row=5, column=1)

        self.lbl_model.grid(row=6, column=0)
        self.ent_model.grid(row=6, column=1)

        self.lbl_spacer.grid(row=7, column=0)
        self.progressb.grid(row=7, column=0, columnspan=2)
        self.progressb.grid_remove()

        self.btn_frame.grid(row=8, column=0, columnspan=2)
        self.start_button.grid(row=0, column=0)
        self.stop_button.grid(row=0, column=1)

        self.cdp_radiobtn.grid(row=9, column=0, pady=3)
        self.lldp_radiobtn.grid(row=9, column=1, pady=3)

        self.lbl_photo.grid(row=9, column=1, pady=3)

        self.ent_switchport.configure(state="readonly")
        self.ent_switch.configure(state="readonly")
        self.ent_ip.configure(state="readonly")
        self.ent_vlan.configure(state="readonly")
        self.ent_voicevlan.configure(state="readonly")
        self.ent_model.configure(state="readonly")

    # start the main loop
    def start(self) -> None:
        self.root.mainloop()

if __name__ == '__main__':
    main()