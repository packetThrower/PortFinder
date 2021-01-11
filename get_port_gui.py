import tkinter
import tkinter.ttk

from scapy.all import *


root = tkinter.Tk()
root.title("  Get Port Info")
root.tk.call('wm', 'iconphoto', root._w, tkinter.PhotoImage(file='otecc.png'))

content = tkinter.Frame(root, padx=5, pady=5)

# instantiate widgets
lbl_switch = tkinter.Label(content, text="Switch: ", fg="blue")
lbl_ip = tkinter.Label(content, text="IP: ", fg="blue")
lbl_switchport = tkinter.Label(content, text="Switchport: ", fg="blue")
lbl_vlan = tkinter.Label(content, text="VLAN: ", fg="blue")
lbl_voicevlan = tkinter.Label(content, text="Voice VLAN: ", fg="blue")
lbl_model = tkinter.Label(content, text="Switch Model: ", fg="blue")
lbl_spacer = tkinter.Label(content, text="")
lbl_sniff_interface = tkinter.Label(content, text="", fg="red")


ent_switch = tkinter.Entry(content, width="30")
ent_ip = tkinter.Entry(content, width="30")
ent_switchport = tkinter.Entry(content, width="30")
ent_vlan = tkinter.Entry(content, width="30")
ent_voicevlan = tkinter.Entry(content, width="30")
ent_model = tkinter.Entry(content, width="30")

progressb = tkinter.ttk.Progressbar(
    content, orient=tkinter.HORIZONTAL, length=200, mode='indeterminate', phase=25)

otec_photo = tkinter.PhotoImage(file="otecc_small.png")
lbl_photo = tkinter.Label(content, image=otec_photo,
                          anchor="e", justify=tkinter.LEFT, width="200")


def run_thread():
    # Run seperate thread so the progress bar will spin
    progressb.grid(row=6, column=0, columnspan=2)
    progressb.start()
    x = threading.Thread(target=run_scan)
    x.start()


def run_scan():
    load_contrib("cdp")
    # sniff for the CDP packet
    sniff(prn=process_packets, store=0,
          filter="ether dst 01:00:0c:cc:cc:cc", count=1)


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

    lbl_sniff_interface.config(text=pkt.sniffed_on)

    ent_switchport.insert(0, pkt['CDPMsgPortID'].iface.decode())
    ent_switch.insert(0, pkt['CDPMsgDeviceID'].val.decode())
    ent_ip.insert(0, pkt['CDPMsgMgmtAddr'].addr[0].addr)
    ent_vlan.insert(0, str(pkt['CDPMsgNativeVLAN'].vlan))
    ent_voicevlan.insert(0, str(pkt['CDPMsgVoIPVLANReply'].vlan))
    ent_model.insert(0, pkt['CDPMsgPlatform'].val.decode())

    progressb.destroy()


start_button = tkinter.Button(
    content, text="Start", state="active", padx=30, pady=5, command=run_thread)

# put widgets into the window
content.grid(row=0, column=0)

lbl_switch.grid(row=0, column=0)
ent_switch.grid(row=0, column=1)

lbl_ip.grid(row=1, column=0)
ent_ip.grid(row=1, column=1)

lbl_switchport.grid(row=2, column=0)
ent_switchport.grid(row=2, column=1)

lbl_vlan.grid(row=3, column=0)
ent_vlan.grid(row=3, column=1)

lbl_voicevlan.grid(row=4, column=0)
ent_voicevlan.grid(row=4, column=1)

lbl_model.grid(row=5, column=0)
ent_model.grid(row=5, column=1)

lbl_spacer.grid(row=6, column=0)

lbl_sniff_interface.grid(row=7, column=0)

start_button.grid(row=7, column=0, columnspan=2)

lbl_photo.grid(row=7, column=1, )

root.mainloop()
