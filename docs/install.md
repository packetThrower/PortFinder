# Install

Pre-built binaries for every release are on the [Releases page](https://github.com/packetThrower/PortFinder/releases).

## macOS

### Homebrew (recommended)

```bash
brew install --cask packetThrower/tap/portfinder
```

This installs `PortFinder.app` to `/Applications` and symlinks the headless CLI to `$(brew --prefix)/bin/portfinder` so [`portfinder`](usage/cli.md) works from any shell. Update with `brew upgrade --cask portfinder`; remove with `brew uninstall --cask portfinder` (add `--zap` to also clear `~/Library/Application Support/PortFinder` and the WebKit cache).

The tap source lives at [packetThrower/homebrew-tap](https://github.com/packetThrower/homebrew-tap), shared with [Baudrun](https://github.com/packetThrower/Baudrun) and other packetThrower projects.

#### Pre-release channel

For early access to alpha / beta / RC builds, install the `@alpha` cask alongside the stable one:

```bash
brew install --cask packetThrower/tap/portfinder@alpha
```

This drops `PortFinder Alpha.app` into `/Applications` and exposes the CLI as `portfinder-alpha`, so the two channels coexist. State (preferences, saved window position) is shared between them. File regressions on the [issue tracker](https://github.com/packetThrower/PortFinder/issues).

For non-root packet capture, click **Install BPF Access** in the app once after install. Until you do, capture works only via `sudo portfinder capture …`.

!!! tip "Already have Wireshark?"
    Wireshark's **ChmodBPF** helper is the same idea — if it's installed, PortFinder will use it and the in-app *Install BPF Access* button isn't needed.

### Manual DMG

Prefer not to use Homebrew? Download `PortFinder_<version>_universal.dmg` from the [Releases page](https://github.com/packetThrower/PortFinder/releases), mount it, and drag PortFinder into `/Applications`.

For non-root capture and a `portfinder` CLI on your `PATH`, also install the BPF helper:

1. Download `PortFinder-BPF-<version>.pkg` from the same release.
2. Double-click to install. macOS will prompt for your password.
3. Click **Install BPF Access** in the app the first time you run it (or re-run the `.pkg`).

The BPF helper also drops a symlink at `/usr/local/bin/portfinder` so the [CLI](usage/cli.md) is callable from any shell.

#### Just the CLI symlink

If you don't need the BPF helper (e.g. you already have Wireshark's, or you're fine running with `sudo`) but you do want `portfinder` on your `PATH`, use the standalone scripts in the repo root:

```bash
# Install the symlink (no BPF, no group changes)
curl -fsSLO https://raw.githubusercontent.com/packetThrower/PortFinder/main/install-cli.sh
sudo sh install-cli.sh

# Remove it later
curl -fsSLO https://raw.githubusercontent.com/packetThrower/PortFinder/main/uninstall-cli.sh
sudo sh uninstall-cli.sh
```

`install-cli.sh` looks for `PortFinder.app` in `/Applications` or `~/Applications` and links the bundle's `portfinder` binary at `/usr/local/bin/portfinder`. `uninstall-cli.sh` removes it again, refusing to touch the symlink unless it actually points back into a `PortFinder.app` bundle.

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

### Scoop (recommended)

```powershell
scoop bucket add packetThrower https://github.com/packetThrower/scoop-bucket
scoop install portfinder
```

This installs `PortFinder.exe`, drops a Start menu shortcut, and exposes the headless CLI on your `PATH` so [`portfinder`](usage/cli.md) works from any shell. Update with `scoop update portfinder`; uninstall with `scoop uninstall portfinder`. The bucket source lives at [packetThrower/scoop-bucket](https://github.com/packetThrower/scoop-bucket).

#### Pre-release channel

The bucket also ships a parallel manifest tracking pre-release tags (alpha / beta / rc). It coexists with the stable install:

```powershell
scoop install portfinder-prerelease
```

The pre-release CLI shim is `portfinder-alpha` and the Start menu shortcut is `PortFinder Alpha`, so the two never collide.

You still need [Npcap](https://npcap.com/#download) for packet capture (see below) — Scoop can't bundle the kernel driver.

### Manual installer

1. Install [Npcap](https://npcap.com/#download) — be sure to tick **"Allow non-administrators to capture"** during install.
2. Download `PortFinder_<version>_x64-setup.exe` (or `_arm64-setup.exe`) from the release.
3. Run the installer.

!!! tip "Already have Wireshark?"
    Wireshark on Windows ships with Npcap, so you may already have it. Open *Programs and Features* and check for **Npcap** — if it's there, you're set.
