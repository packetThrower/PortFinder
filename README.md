# get_switch_info

Run these scripts to get lldp or cdp info from a switch that your computer is plugged into. get_port.py uses CDP.

* Written for linux environment.

### TODO:

* Create Script for LLDP (Aruba will use LLDP)
* Make multi-platform (Win Version)

## How to

Be sure to run with Wifi desabled on the device. tcpdump will get confused.

- Create a python virtual evironment:

        python3 -m venv --copies venv

- Activate environment:

        source venv/bin/activate

- Install dependencies:

        pip3 install -r requirements.txt

- Run the application:

        python3 get_port.py

- Alternative execution:

        /venv/bin/python3 get_port.py

## Files

`get_port.py` - Python version of the Shell script with some pretty printing.

`get_ports.sh` - Shell script for doing the samething as the python script. (Only works on Linux and tested against Ubuntu)

## To Contribute

- Be sure to install TK for the GUI dependencies:

        sudo apt install python3-tk

