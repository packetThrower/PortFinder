<script lang="ts">
    import './App.css';
    import { invoke } from '@tauri-apps/api/core';
    import { type as osType } from '@tauri-apps/plugin-os';
    import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
    import { _, locale } from 'svelte-i18n';
    import { LOCALES, setLocale, type LocaleCode } from './i18n';
    import type {
        InterfaceInfo,
        CaptureRequest,
        CaptureResult,
        PrivilegeStatus,
    } from './bindings';

    // Base window height matches tauri.conf.json. When a privilege warning
    // banner is rendered we grow the window to fit it; when it goes away
    // (e.g. the user clicks Install BPF Access and capture access becomes
    // available) we shrink back.
    const BASE_HEIGHT = 460;
    const BASE_WIDTH = 400;

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
    let protocol = $state<'CDP' | 'LLDP' | 'MNDP'>('LLDP');
    let isCapturing = $state(false);
    let result = $state<CaptureResult | null>(null);
    // Reactive status: store the i18n key (+ optional values) instead of
    // the resolved string, so the visible status text re-renders when the
    // user changes language mid-session.
    let statusKey = $state('status.ready');
    let statusValues = $state<Record<string, string>>({});
    let error = $state('');

    // Click-to-copy state for result values. Borrowed from Direction B
    // of the design comparison: clicking a value writes it to the
    // clipboard and shows a transient checkmark. Saves the
    // highlight-and-cmd-c step for users pasting into tickets / Slack.
    let copiedKey = $state<string | null>(null);
    let copyTimer: ReturnType<typeof setTimeout> | null = null;
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

    /**
     * Decide how a captured value renders. The Rust parsers populate
     * any field that wasn't in the packet with the literal "N/A"
     * sentinel; the GUI re-interprets that as "not advertised" italic
     * faded text so absence reads as honest information rather than
     * a shrug. Empty string is treated the same way.
     */
    function valueOrAbsent(v: string): { text: string; absent: boolean } {
        if (!v || v === 'N/A') {
            return { text: $_('result.notAdvertised'), absent: true };
        }
        return { text: v, absent: false };
    }

    async function copyValue(key: string, text: string) {
        if (!text) return;
        try {
            await navigator.clipboard.writeText(text);
            copiedKey = key;
            if (copyTimer) clearTimeout(copyTimer);
            copyTimer = setTimeout(() => (copiedKey = null), 1200);
        } catch {
            // Some platform / focus combinations (e.g. WebView before
            // initial focus) can fail clipboard writes silently. The
            // value is already on screen so a no-op is fine.
        }
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
                error = $_('status.loadInterfacesFailed', {
                    values: { detail: String(err) },
                });
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

    // Resize the window to fit the privilege-warning banner when it's
    // visible. Re-runs whenever privStatus changes (initial load, after
    // BPF install, etc.).
    $effect(() => {
        // Read privStatus so Svelte tracks it as a dependency.
        void privStatus;
        // Wait one frame so the DOM has rendered the banner (or removed it)
        // before measuring.
        requestAnimationFrame(() => {
            const banner = document.querySelector('.privilege-warning') as HTMLElement | null;
            const extra = banner ? banner.offsetHeight + 12 : 0;
            try {
                getCurrentWindow().setSize(
                    new LogicalSize(BASE_WIDTH, BASE_HEIGHT + extra),
                );
            } catch {
                // Non-Tauri preview, or set_size denied — silently no-op.
            }
        });
    });

    async function handleInstallBPF() {
        isInstalling = true;
        error = '';
        try {
            await InstallBPFHelper();
            statusKey = 'status.bpfInstalled';
            statusValues = {};
            refreshPrivileges();
        } catch (err: unknown) {
            error = typeof err === 'string'
                ? err
                : err instanceof Error ? err.message : $_('status.installFailed');
        } finally {
            isInstalling = false;
        }
    }

    async function handleStart() {
        isCapturing = true;
        error = '';
        result = null;
        statusKey = 'status.capturing';
        statusValues = { protocol };

        try {
            const res = await StartCapture({
                interfaceName: selectedInterface,
                protocol: protocol,
            });
            if (res) {
                result = res;
                statusKey = 'status.complete';
                statusValues = {};
            }
        } catch (err: unknown) {
            const msg = typeof err === 'string'
                ? err
                : err instanceof Error ? err.message : $_('status.captureFailed');
            if (msg.includes('cancelled')) {
                statusKey = 'status.stopped';
                statusValues = {};
            } else {
                error = msg;
                statusKey = 'status.error';
                statusValues = {};
            }
        } finally {
            isCapturing = false;
        }
    }

    function handleStop() {
        StopCapture();
        statusKey = 'status.stopping';
        statusValues = {};
    }

    // Labels for the language picker. Native names so each entry is
    // recognizable to a speaker of that language regardless of the
    // currently active locale.
    const LOCALE_LABELS: Record<LocaleCode, string> = {
        en: 'English',
        es: 'Español',
        fr: 'Français',
        de: 'Deutsch',
    };
</script>

<div class="app">
    {#if privStatus && !privStatus.hasAccess}
        {#if privStatus.platform === 'darwin' && privStatus.canInstall}
            <div class="privilege-warning">
                <div>{$_('privilege.macosBpf')}</div>
                <button
                    class="install-btn"
                    onclick={handleInstallBPF}
                    disabled={isInstalling}
                >
                    {isInstalling ? $_('status.installing') : $_('privilege.installBpf')}
                </button>
            </div>
        {:else if privStatus.platform === 'linux'}
            <div class="privilege-warning">
                {$_('privilege.linuxSudo')}
            </div>
        {:else if privStatus.platform === 'windows' && !privStatus.npcapInstalled}
            <div class="privilege-warning">
                <div>{$_('privilege.windowsNpcap')}</div>
                <a
                    class="install-btn"
                    href="https://npcap.com/#download"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    {$_('privilege.downloadNpcap')}
                </a>
                <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                    {$_('privilege.windowsNpcapHint')}
                </div>
            </div>
        {:else if privStatus.platform === 'windows' && !privStatus.npcapNonAdmin}
            <div class="privilege-warning">
                <div>{$_('privilege.windowsNpcapAdmin')}</div>
                <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                    {$_('privilege.windowsNpcapAdminHint')}
                </div>
            </div>
        {:else}
            <div class="privilege-warning">
                {$_('privilege.elevated')}
            </div>
        {/if}
    {/if}

    <!-- Controls card: configure capture -->
    <section class="card">
        <div class="form-group">
            <label for="nic-select">{$_('controls.interface')}</label>
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
                            {iface.name === ''
                                ? $_('controls.sniffAll')
                                : (iface.description || iface.name)}{compact ? ` (${compact})` : ''}
                        </option>
                    {/each}
                </select>
                <button
                    type="button"
                    class="refresh-btn"
                    onclick={refreshInterfaces}
                    disabled={isCapturing}
                    title={$_('controls.refresh')}
                    aria-label={$_('controls.refresh')}
                >
                    ↻
                </button>
            </div>
        </div>

        <label class="switch-row">
            <span class="switch-label">{$_('controls.onlyWithIp')}</span>
            <input
                type="checkbox"
                role="switch"
                class="switch"
                bind:checked={showOnlyWithIPs}
                disabled={isCapturing}
            />
        </label>

        <div class="form-group">
            <label for="protocol-select">{$_('controls.protocol')}</label>
            <select
                id="protocol-select"
                bind:value={protocol}
                disabled={isCapturing}
            >
                <option value="LLDP">LLDP</option>
                <option value="CDP">CDP</option>
                <option value="MNDP">MNDP</option>
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
                {$_('controls.start')}
            </button>
            <button
                onclick={handleStop}
                disabled={!isCapturing}
            >
                {$_('controls.stop')}
            </button>
        </div>
    </section>

    <!-- Result card: captured switch info -->
    <section class="card">
        {#if result}
            {@const fields: [string, string, string][] = [
                ['switch', $_('result.switch'), result.switchName],
                ['ip', $_('result.ip'), result.switchIp],
                ['port', $_('result.port'), result.switchPort],
                ['vlan', $_('result.vlan'), result.nativeVlan],
                ['voiceVlan', $_('result.voiceVlan'), result.voiceVlan],
                ['mtu', $_('result.mtu'), result.mtu],
                ['model', $_('result.model'), result.switchModel],
            ]}
            <dl class="result-list">
                {#each fields as [key, label, raw]}
                    {@const v = valueOrAbsent(raw)}
                    <dt>{label}</dt>
                    <dd>
                        {#if v.absent}
                            <span class="value absent">{v.text}</span>
                        {:else}
                            <button
                                type="button"
                                class="value value-copy"
                                class:copied={copiedKey === key}
                                onclick={() => copyValue(key, v.text)}
                                title={$_('result.copyTitle', { values: { value: v.text } })}
                            >
                                <span class="value-text">{v.text}</span>
                                {#if copiedKey === key}
                                    <span class="copied-mark" aria-hidden="true">✓</span>
                                {/if}
                            </button>
                        {/if}
                    </dd>
                {/each}
            </dl>
        {:else}
            <p class="empty-state">
                {$_('result.empty')}
            </p>
        {/if}
    </section>

    <div class="status-text" class:error-text={!!error}>
        {error || $_(statusKey, { values: statusValues })}
    </div>

    <div class="footer-row">
        {#if version}
            <span class="version-text">v{version}</span>
        {/if}
        <select
            class="locale-picker"
            value={$locale ?? 'en'}
            onchange={(e) => setLocale((e.currentTarget as HTMLSelectElement).value as LocaleCode)}
            aria-label="Language"
        >
            {#each LOCALES as code}
                <option value={code}>{LOCALE_LABELS[code]}</option>
            {/each}
        </select>
    </div>
</div>
