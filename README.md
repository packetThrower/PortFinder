# get_switch_info

Run these scripts to get lldp or cdp info from a switch that your computer is plugged into. get_port.py uses CDP.

* Written for linux environment.

## Files

`PortFinder.py` - Main script with GUI. This can be compiled with pyinstaller for binaries, or ran as a script with python PortFiner. On Linux this script requires sudo privilages. It may require admin rights on Windows (untested).

`get_port.py` - Standalone script with no GUI. Pretty prints a table with the results. (May be merged into PortFinder in the future). Untested on Windows. (only CDP)

`get_ports.sh` = Shell script for running tcpdump to get the same info. (only CDP)

## TODO:

* Merge get_port.py into PortFinder.py for standalone arg.
* Enable LLDP in get_port.py and get_ports.sh

## How to

PortFinder.exe and linux binaries are easy. Doubleclick and go. When the GUI appears, you can select a NIC if you know which one you want to sniff on. The default option "Sniff all Interfaces" does what it says. The default is the safest. 

At the bottom of the window, you will see two radio buttons. You can select CDP or LLDP. CDP is for when your device is connected to a Cisco switch. LLDP may work agaist Cisco, but it has to be enabled on Cisco. LLDP will be used for Aruba. 

Click start. When the sniff is finished, the results will be displayed. Strings from the text boxes can be copied and pasted to other documents.

Stop will stop the scan. Useful if you have selected the wrong interface.

* The other scripts can be ran from the command line.

# Contributing

## To Contribute

- Be sure to install TK for the GUI dependencies on linux:

        sudo apt install python3-tk


- Create a python virtual evironment:

        python3 -m venv --copies venv

- Activate environment:

        source venv/bin/activate

- Install dependencies:

        pip3 install -r requirements.txt

- Run the application:

        python3 get_port.py

        or 

        python3 PortFinder.py

- Alternative execution:

        /venv/bin/python3 get_port.py

        or 

        /venv/bin/python3 PortFinder.py

Now you can edit and modify.


# Build

## Requirements for building

- Windows

        (Microsoft Visual C++ 14.0)[https://visualstudio.microsoft.com/visual-cpp-build-tools/] (Warning: Large download) - Required for netifaces.

- Linux

        sudo apt install python3-tk  - Required for the GUI (tkinter)

### Build with pyinstaller

Install pyinstaller

        pip install pyinstaller

- linux

        pyinstaller --onefile --add-data="*.png:." PortFinder.py

- Windows

        pyinstaller.exe --onefile --noconsole --noupx --add-data="*.png;." .\PortFinder.py
