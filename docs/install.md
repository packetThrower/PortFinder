# Install

Pre-built binaries for every release are on the [Releases page](https://github.com/packetThrower/PortFinder/releases).

## macOS

Download `PortFinder_<version>_universal.dmg`, mount it, and drag PortFinder into `/Applications`.

For non-root packet capture, also install the BPF helper:

1. Download `PortFinder-BPF-<version>.pkg` from the same release.
2. Double-click to install. macOS will prompt for your password.
3. Click **Install BPF Access** in the app the first time you run it (or re-run the `.pkg`).

!!! tip "Already have Wireshark?"
    Wireshark's **ChmodBPF** helper is the same idea — if it's installed, PortFinder will use it.

## Linux

Pick the package that matches your distro:

=== ".deb (Debian / Ubuntu)"

    ```bash
    sudo apt install ./PortFinder_<version>_amd64.deb
    ```

=== ".rpm (Fedora / RHEL)"

    ```bash
    sudo dnf install ./PortFinder-<version>-1.x86_64.rpm
    ```

=== ".AppImage (any distro)"

    ```bash
    chmod +x PortFinder_<version>_amd64.AppImage
    ./PortFinder_<version>_amd64.AppImage
    ```

The `.deb` and `.rpm` packages run a postinstall hook that grants `CAP_NET_RAW` to the binary, so packet capture works without `sudo`.

## Windows

1. Install [Npcap](https://npcap.com/#download) — be sure to tick **"Allow non-administrators to capture"** during install.
2. Download `PortFinder_<version>_x64-setup.exe` from the release.
3. Run the installer.
