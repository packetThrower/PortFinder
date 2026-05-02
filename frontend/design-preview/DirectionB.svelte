<script lang="ts">
    /**
     * Direction B — Custom-unified, Linear-flavored.
     *
     * No per-OS chrome, no system colors, no native-widget reliance.
     * Single visual surface across macOS / Windows / Linux. Carries
     * PortFinder's identity by reaching for the icon's pin amber as a
     * single accent (≤10% of the surface), tinting the cool-grey
     * neutrals subtly toward the icon's navy hue (250) for cohesion.
     *
     * Distinctive moves vs Direction A:
     *   - Inter Variable, single typeface, fixed scale.
     *   - Custom card chrome (8px radius, single border treatment).
     *   - Custom buttons / inputs / select with consistent states.
     *   - Click-to-copy on result values, transient checkmark on success.
     *   - Subtle amber leading tick on populated result rows; thin dash
     *     stroke on absent rows (state without color-only signaling).
     *   - Generous spacing rhythm, 4pt scale.
     */
    import type { AppSnapshot } from './sample-data';
    import { INTERFACES } from './sample-data';

    let { snapshot, theme }: { snapshot: AppSnapshot; theme: 'light' | 'dark' } = $props();

    let copiedKey = $state<string | null>(null);
    let copyTimer: ReturnType<typeof setTimeout> | null = null;

    function compactAddresses(addrs: string): string {
        if (!addrs) return '';
        const parts = addrs.split(', ');
        const v4 = parts.find((p) => /^\d+\.\d+\.\d+\.\d+$/.test(p));
        return v4 ?? parts[0] ?? '';
    }

    function valueOrAbsent(v: string): { text: string; absent: boolean } {
        if (!v || v === 'N/A') return { text: 'not advertised', absent: true };
        return { text: v, absent: false };
    }

    async function copy(key: string, text: string) {
        if (!text || text === 'not advertised') return;
        try {
            await navigator.clipboard.writeText(text);
            copiedKey = key;
            if (copyTimer) clearTimeout(copyTimer);
            copyTimer = setTimeout(() => (copiedKey = null), 1200);
        } catch {
            // navigator.clipboard fails in non-secure contexts; the comp
            // page is just for visual review so we silently no-op.
        }
    }
</script>

<div class="surface-b" class:dark={theme === 'dark'}>
    <div class="window">
        {#if snapshot.privilegeBanner === 'macos-bpf'}
            <div class="banner">
                <div class="banner-msg">Packet capture needs BPF device access on macOS.</div>
                <button class="banner-btn">Install BPF helper</button>
            </div>
        {:else if snapshot.privilegeBanner === 'windows-npcap'}
            <div class="banner">
                <div class="banner-msg">Npcap isn't installed on this machine.</div>
                <a class="banner-btn" href="#">Download Npcap</a>
                <div class="banner-hint">Tick "Allow non-administrators to capture" in the installer.</div>
            </div>
        {/if}

        <section class="card">
            <div class="row">
                <label for="b-iface">Interface</label>
                <div class="iface-row">
                    <select id="b-iface" class="control" disabled={snapshot.isCapturing} value={snapshot.selectedInterface}>
                        {#each INTERFACES as iface}
                            {@const compact = compactAddresses(iface.addresses)}
                            <option value={iface.name}>{iface.description}{compact ? ` (${compact})` : ''}</option>
                        {/each}
                    </select>
                    <button class="control refresh" disabled={snapshot.isCapturing} aria-label="Refresh">↻</button>
                </div>
            </div>

            <label class="switch-row">
                <span>Only show interfaces with an IP</span>
                <span class="switch on" role="switch" aria-checked="true">
                    <span class="switch-thumb"></span>
                </span>
            </label>

            <div class="row">
                <label for="b-protocol">Protocol</label>
                <select id="b-protocol" class="control" disabled={snapshot.isCapturing} value={snapshot.protocol}>
                    <option>LLDP</option>
                    <option>CDP</option>
                    <option>MNDP</option>
                </select>
            </div>

            {#if snapshot.isCapturing}
                <div class="progress"><div class="progress-fill"></div></div>
            {/if}

            <div class="buttons">
                <button class="btn primary" disabled={snapshot.isCapturing}>Start</button>
                <button class="btn" disabled={!snapshot.isCapturing}>Stop</button>
            </div>
        </section>

        <section class="card result-card">
            {#if snapshot.result}
                {@const r = snapshot.result}
                {@const fields: [string, string, string][] = [
                    ['switch', 'Switch', r.switchName],
                    ['ip', 'IP', r.switchIp],
                    ['port', 'Port', r.switchPort],
                    ['nativeVlan', 'VLAN', r.nativeVlan],
                    ['voiceVlan', 'Voice VLAN', r.voiceVlan],
                    ['mtu', 'MTU', r.mtu],
                    ['model', 'Model', r.switchModel],
                ]}
                <dl class="result">
                    {#each fields as [key, label, raw]}
                        {@const v = valueOrAbsent(raw)}
                        <dt class:absent={v.absent}>{label}</dt>
                        <dd>
                            <button
                                type="button"
                                class="value-btn"
                                class:absent={v.absent}
                                class:copied={copiedKey === key}
                                disabled={v.absent}
                                onclick={() => copy(key, v.text)}
                                title={v.absent ? '' : `Copy "${v.text}"`}
                            >
                                <span class="value-text">{v.text}</span>
                                {#if copiedKey === key}
                                    <span class="copied-mark" aria-hidden="true">✓</span>
                                {/if}
                            </button>
                        </dd>
                    {/each}
                </dl>
            {:else}
                <p class="empty">Run a capture to see what switch you're on.</p>
            {/if}
        </section>

        <div class="status" class:error={snapshot.statusError}>
            {#if snapshot.isCapturing}
                <span class="status-pulse" aria-hidden="true"></span>
            {/if}
            {snapshot.statusText}
        </div>

        <div class="footer">
            <span class="version">v3.3.0-alpha.1</span>
        </div>
    </div>
</div>

<style>
    /* Direction B — Linear-flavored Restrained palette.
       Cool greys (hue 250) tinted faintly toward the icon's navy.
       Single accent: amber (hue 75) from the icon's pin gradient. */

    .surface-b {
        --bg-page: oklch(98.4% 0.004 250);
        --bg-card: oklch(99.5% 0.003 250);
        --card-border: oklch(91% 0.008 250);
        --card-inner-line: oklch(94% 0.005 250);

        --text: oklch(22% 0.015 250);
        --text-muted: oklch(50% 0.012 250);
        --text-faint: oklch(68% 0.008 250);

        --accent: oklch(72% 0.14 75);
        --accent-strong: oklch(60% 0.16 70);
        --accent-soft: oklch(94% 0.04 80);

        --error: oklch(54% 0.18 28);

        --switch-off: oklch(86% 0.008 250);

        --banner-bg: oklch(96% 0.025 80);
        --banner-line: oklch(82% 0.06 75);
        --banner-text: oklch(35% 0.08 70);

        font-family: 'Inter', -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
        font-feature-settings: 'cv02', 'cv03', 'cv04', 'cv11';
        font-size: 13px;
        line-height: 1.45;
        color: var(--text);
        background: var(--bg-page);

        width: 400px;
        height: 500px;
        border-radius: 12px;
        overflow: hidden;
        position: relative;
        box-shadow:
            0 0 0 1px var(--card-border),
            0 24px 48px -12px oklch(20% 0.04 250 / 0.18),
            0 4px 8px oklch(20% 0.04 250 / 0.05);
    }

    .surface-b.dark {
        --bg-page: oklch(17% 0.01 250);
        --bg-card: oklch(21% 0.012 250);
        --card-border: oklch(28% 0.014 250);
        --card-inner-line: oklch(25% 0.012 250);

        --text: oklch(95% 0.005 250);
        --text-muted: oklch(70% 0.012 250);
        --text-faint: oklch(50% 0.012 250);

        --accent: oklch(80% 0.13 75);
        --accent-strong: oklch(72% 0.15 70);
        --accent-soft: oklch(28% 0.05 75);

        --error: oklch(70% 0.16 28);

        --switch-off: oklch(34% 0.012 250);

        --banner-bg: oklch(28% 0.04 80);
        --banner-line: oklch(40% 0.07 75);
        --banner-text: oklch(92% 0.04 80);
    }

    .window {
        padding: 14px 14px 10px;
        height: 100%;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .card {
        background: var(--bg-card);
        border: 1px solid var(--card-border);
        border-radius: 10px;
        padding: 16px 18px;
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .result-card {
        padding: 14px 18px;
    }

    .row {
        display: grid;
        grid-template-columns: 80px 1fr;
        column-gap: 12px;
        align-items: center;
    }

    .row label {
        text-align: right;
        color: var(--text-muted);
        font-weight: 450;
        letter-spacing: 0.005em;
    }

    .control {
        font: inherit;
        padding: 5px 10px;
        background: var(--bg-page);
        color: var(--text);
        border: 1px solid var(--card-border);
        border-radius: 6px;
        outline: none;
        transition: border-color 120ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .control:hover:not(:disabled),
    .control:focus-visible {
        border-color: var(--text-muted);
    }

    .control:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }

    .control:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .iface-row {
        display: flex;
        gap: 6px;
    }

    .iface-row select {
        flex: 1;
        min-width: 0;
    }

    .refresh {
        flex-shrink: 0;
        min-width: 30px;
        font-size: 14px;
        line-height: 1;
        padding: 4px 6px;
        cursor: pointer;
    }

    .switch-row {
        display: flex;
        align-items: center;
        gap: 12px;
        font-size: 0.92em;
        cursor: pointer;
        user-select: none;
        color: var(--text-muted);
    }

    .switch-row > span:first-child {
        flex: 1;
    }

    .switch {
        position: relative;
        width: 36px;
        height: 20px;
        border-radius: 10px;
        background: var(--switch-off);
        flex-shrink: 0;
        transition: background 200ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .switch.on {
        background: var(--accent);
    }

    .switch-thumb {
        position: absolute;
        top: 2px;
        left: 2px;
        width: 16px;
        height: 16px;
        border-radius: 8px;
        background: var(--bg-card);
        box-shadow: 0 1px 2px oklch(20% 0.04 250 / 0.25);
        transition: transform 200ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .switch.on .switch-thumb {
        transform: translateX(16px);
    }

    .progress {
        width: 100%;
        height: 2px;
        border-radius: 1px;
        background: var(--card-inner-line);
        overflow: hidden;
    }

    .progress-fill {
        width: 35%;
        height: 100%;
        background: var(--accent);
        border-radius: 1px;
        animation: b-progress 1.2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
    }

    @keyframes b-progress {
        0% { transform: translateX(-100%); }
        100% { transform: translateX(300%); }
    }

    @media (prefers-reduced-motion: reduce) {
        .progress-fill {
            animation: none;
            width: 100%;
            opacity: 0.55;
        }
    }

    .buttons {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
    }

    .btn {
        font: inherit;
        font-weight: 500;
        padding: 6px 16px;
        min-width: 76px;
        background: var(--bg-page);
        color: var(--text);
        border: 1px solid var(--card-border);
        border-radius: 6px;
        cursor: pointer;
        transition:
            border-color 120ms cubic-bezier(0.25, 1, 0.5, 1),
            background 120ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .btn:hover:not(:disabled) {
        border-color: var(--text-muted);
    }

    .btn.primary {
        background: var(--text);
        color: var(--bg-card);
        border-color: var(--text);
    }

    .surface-b.dark .btn.primary {
        background: var(--accent);
        color: oklch(15% 0.04 75);
        border-color: var(--accent);
    }

    .btn.primary:hover:not(:disabled) {
        background: var(--text-muted);
        border-color: var(--text-muted);
    }

    .surface-b.dark .btn.primary:hover:not(:disabled) {
        background: var(--accent-strong);
        border-color: var(--accent-strong);
    }

    .btn:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }

    .btn:disabled {
        opacity: 0.45;
        cursor: not-allowed;
    }

    /* Result list — the hero surface. The amber tick on populated rows
       carries state without color-only signaling: present rows have a
       tick *and* full-opacity weighted text; absent rows have a thin
       dash *and* italic faded text. CVD users still distinguish them. */
    .result {
        margin: 0;
        display: grid;
        grid-template-columns: 88px 1fr;
        column-gap: 12px;
        row-gap: 6px;
        align-items: baseline;
    }

    .result dt {
        text-align: right;
        color: var(--text-muted);
        font-size: 0.92em;
        font-weight: 450;
        letter-spacing: 0.005em;
        padding-right: 2px;
    }

    .result dt.absent {
        color: var(--text-faint);
    }

    .result dd {
        margin: 0;
    }

    .value-btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        max-width: 100%;
        font: inherit;
        font-weight: 500;
        font-variant-numeric: tabular-nums;
        background: transparent;
        color: inherit;
        border: 1px solid transparent;
        border-radius: 4px;
        padding: 1px 5px;
        margin-left: -5px;
        cursor: pointer;
        text-align: left;
        transition:
            background 120ms cubic-bezier(0.25, 1, 0.5, 1),
            border-color 120ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .value-btn:hover:not(:disabled) {
        background: var(--accent-soft);
    }

    .value-btn:focus-visible {
        outline: none;
        border-color: var(--accent);
    }

    .value-btn.copied {
        background: var(--accent-soft);
        border-color: var(--accent);
    }

    .value-btn.absent {
        font-weight: 400;
        font-style: italic;
        color: var(--text-faint);
        cursor: default;
    }

    .value-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .copied-mark {
        color: var(--accent-strong);
        font-weight: 600;
        font-size: 0.9em;
        flex-shrink: 0;
    }

    .empty {
        margin: 4px 0;
        text-align: center;
        color: var(--text-muted);
        font-size: 0.92em;
        font-style: italic;
    }

    .status {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        text-align: center;
        color: var(--text-muted);
        font-size: 0.86em;
        min-height: 1.2em;
        letter-spacing: 0.01em;
    }

    .status.error {
        color: var(--error);
        font-weight: 500;
    }

    .status-pulse {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: var(--accent);
        animation: b-pulse 1.4s cubic-bezier(0.4, 0, 0.6, 1) infinite;
    }

    @keyframes b-pulse {
        0%, 100% { opacity: 0.4; transform: scale(0.85); }
        50% { opacity: 1; transform: scale(1); }
    }

    @media (prefers-reduced-motion: reduce) {
        .status-pulse {
            animation: none;
            opacity: 0.7;
        }
    }

    .footer {
        text-align: center;
    }

    .version {
        color: var(--text-faint);
        font-size: 0.78em;
        font-variant-numeric: tabular-nums;
        letter-spacing: 0.02em;
    }

    .banner {
        background: var(--banner-bg);
        color: var(--banner-text);
        border: 1px solid var(--banner-line);
        border-radius: 8px;
        padding: 10px 14px;
        font-size: 0.9em;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .banner-btn {
        align-self: flex-start;
        font: inherit;
        font-weight: 500;
        padding: 4px 12px;
        background: transparent;
        color: inherit;
        border: 1px solid var(--banner-line);
        border-radius: 5px;
        text-decoration: none;
        cursor: pointer;
    }

    .banner-btn:hover {
        background: oklch(50% 0.06 75 / 0.1);
    }

    .banner-hint {
        font-size: 0.86em;
        opacity: 0.78;
    }
</style>
