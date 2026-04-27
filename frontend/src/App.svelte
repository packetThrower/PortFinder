<script lang="ts">
    import './App.css';
    import { invoke } from '@tauri-apps/api/core';
    import { type as osType } from '@tauri-apps/plugin-os';
    import type {
        InterfaceInfo,
        CaptureRequest,
        CaptureResult,
        PrivilegeStatus,
    } from './types';

    const GetInterfaces = () =>
        invoke<InterfaceInfo[]>('get_interfaces');
    const StartCapture = (req: CaptureRequest) =>
        invoke<CaptureResult | null>('start_capture', { request: req });
    const StopCapture = () =>
        invoke<void>('stop_capture');
    const GetVersion = () =>
        invoke<string>('get_version');
    const GetPrivilegeStatus = () =>
        invoke<PrivilegeStatus>('get_privilege_status');
    const InstallBPFHelper = () =>
        invoke<void>('install_bpf_helper');

    let interfaces = $state<InterfaceInfo[]>([]);
    let selectedInterface = $state('');
    let protocol = $state<'CDP' | 'LLDP'>('LLDP');
    let isCapturing = $state(false);
    let result = $state<CaptureResult | null>(null);
    let status = $state('Ready');
    let error = $state('');
    let privStatus = $state<PrivilegeStatus | null>(null);
    let isInstalling = $state(false);
    let version = $state('');
    let showOnlyWithIPs = $state(true);

    const filteredInterfaces = $derived(
        showOnlyWithIPs
            ? interfaces.filter((iface) => iface.name === '' || iface.hasIp)
            : interfaces
    );

    /**
     * Pull the first IPv4 out of the comma-separated address string.
     * The full address list still ships in the data — we just hide the
     * IPv6 noise from the dropdown label so common cases stay readable.
     */
    function compactAddresses(addrs: string): string {
        if (!addrs) return '';
        const parts = addrs.split(', ');
        const v4 = parts.find((p) => /^\d+\.\d+\.\d+\.\d+$/.test(p));
        return v4 ?? parts[0] ?? '';
    }

    function refreshPrivileges() {
        GetPrivilegeStatus().then((s) => (privStatus = s));
    }

    function refreshInterfaces() {
        GetInterfaces()
            .then((ifaces) => {
                interfaces = ifaces || [];
            })
            .catch((err) => {
                error = 'Failed to load interfaces: ' + err;
            });
    }

    $effect(() => {
        refreshInterfaces();
        refreshPrivileges();
        GetVersion().then((v) => (version = v));
        // Tag the root element with the OS so per-platform CSS rules can
        // apply native fonts, colors, and spacing.
        try {
            document.documentElement.dataset.os = osType();
        } catch {
            // Non-Tauri preview — leave unset; CSS falls back to defaults.
        }
    });

    async function handleInstallBPF() {
        isInstalling = true;
        error = '';
        try {
            await InstallBPFHelper();
            status = 'BPF access installed. You may need to restart the app.';
            refreshPrivileges();
        } catch (err: unknown) {
            error = typeof err === 'string'
                ? err
                : err instanceof Error ? err.message : 'Installation failed';
        } finally {
            isInstalling = false;
        }
    }

    async function handleStart() {
        isCapturing = true;
        error = '';
        result = null;
        status = 'Capturing ' + protocol + ' packets...';

        try {
            const res = await StartCapture({
                interfaceName: selectedInterface,
                protocol: protocol,
            });
            if (res) {
                result = res;
                status = 'Capture complete';
            }
        } catch (err: unknown) {
            const msg = typeof err === 'string'
                ? err
                : err instanceof Error ? err.message : 'Capture failed';
            if (msg.includes('cancelled')) {
                status = 'Capture stopped';
            } else {
                error = msg;
                status = 'Error';
            }
        } finally {
            isCapturing = false;
        }
    }

    function handleStop() {
        StopCapture();
        status = 'Stopping...';
    }
</script>

<div class="app">
    {#if privStatus && !privStatus.hasAccess}
        {#if privStatus.platform === 'darwin' && privStatus.canInstall}
            <div class="privilege-warning">
                <div>Packet capture requires BPF device access.</div>
                <button
                    class="install-btn"
                    onclick={handleInstallBPF}
                    disabled={isInstalling}
                >
                    {isInstalling ? 'Installing...' : 'Install BPF Access'}
                </button>
            </div>
        {:else if privStatus.platform === 'linux'}
            <div class="privilege-warning">
                Run with sudo or install the .deb/.rpm package (sets CAP_NET_RAW).
            </div>
        {:else if privStatus.platform === 'windows' && !privStatus.npcapInstalled}
            <div class="privilege-warning">
                <div>Npcap is required for packet capture.</div>
                <a
                    class="install-btn"
                    href="https://npcap.com/#download"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    Download Npcap
                </a>
                <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                    Enable "Allow non-administrators to capture" during install.
                </div>
            </div>
        {:else if privStatus.platform === 'windows' && !privStatus.npcapNonAdmin}
            <div class="privilege-warning">
                <div>Npcap is installed but requires admin privileges.</div>
                <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                    Reinstall Npcap with "Allow non-administrators to capture" enabled, or run as Administrator.
                </div>
            </div>
        {:else}
            <div class="privilege-warning">
                Elevated privileges required for packet capture.
            </div>
        {/if}
    {/if}

    <!-- Controls card: configure capture -->
    <section class="card">
        <div class="form-group">
            <label for="nic-select">Interface:</label>
            <div class="nic-row">
                <select
                    id="nic-select"
                    bind:value={selectedInterface}
                    disabled={isCapturing}
                    title={interfaces.find((i) => i.name === selectedInterface)?.addresses ?? ''}
                >
                    {#each filteredInterfaces as iface (iface.name || '__all__')}
                        {@const compact = compactAddresses(iface.addresses)}
                        <option value={iface.name} title={iface.addresses}>
                            {iface.description || iface.name || 'Sniff all Interfaces'}{compact ? ` (${compact})` : ''}
                        </option>
                    {/each}
                </select>
                <button
                    type="button"
                    class="refresh-btn"
                    onclick={refreshInterfaces}
                    disabled={isCapturing}
                    title="Refresh interface list"
                    aria-label="Refresh interface list"
                >
                    ↻
                </button>
            </div>
        </div>

        <label class="switch-row">
            <span class="switch-label">Only show interfaces with an IP</span>
            <input
                type="checkbox"
                role="switch"
                class="switch"
                bind:checked={showOnlyWithIPs}
                disabled={isCapturing}
            />
        </label>

        <div class="form-group">
            <label for="protocol-select">Protocol:</label>
            <select
                id="protocol-select"
                bind:value={protocol}
                disabled={isCapturing}
            >
                <option value="LLDP">LLDP</option>
                <option value="CDP">CDP</option>
            </select>
        </div>

        {#if isCapturing}
            <div class="progress-bar">
                <div class="progress-fill"></div>
            </div>
        {/if}

        <div class="button-row">
            <!-- svelte-ignore a11y_autofocus -->
            <button
                onclick={handleStart}
                disabled={isCapturing}
                autofocus
            >
                Start
            </button>
            <button
                onclick={handleStop}
                disabled={!isCapturing}
            >
                Stop
            </button>
        </div>
    </section>

    <!-- Result card: captured switch info -->
    <section class="card">
        {#if result}
            <dl class="result-list">
                <dt>Switch</dt><dd>{result.switchName || '—'}</dd>
                <dt>IP</dt><dd>{result.switchIp || '—'}</dd>
                <dt>Port</dt><dd>{result.switchPort || '—'}</dd>
                <dt>VLAN</dt><dd>{result.nativeVlan || '—'}</dd>
                <dt>Voice VLAN</dt><dd>{result.voiceVlan || '—'}</dd>
                <dt>MTU</dt><dd>{result.mtu || '—'}</dd>
                <dt>Model</dt><dd>{result.switchModel || '—'}</dd>
            </dl>
        {:else}
            <p class="empty-state">
                No capture yet. Choose an interface and click Start.
            </p>
        {/if}
    </section>

    <div class="status-text" class:error-text={!!error}>
        {error || status}
    </div>

    {#if version}
        <div class="version-text">v{version}</div>
    {/if}
</div>
