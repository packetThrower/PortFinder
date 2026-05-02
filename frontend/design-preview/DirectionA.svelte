<script lang="ts">
    /**
     * Direction A — Native-refined.
     *
     * The current "Settings on Every Machine" approach, polished. Same
     * macOS Tahoe card chrome the production app already uses; same
     * system colors; native-rendered widgets. Differences from the
     * shipping version are entirely in the disciplined application of
     * existing primitives:
     *
     *   - Tighter spacing rhythm (single 4px-base scale, no per-OS drift).
     *   - Clearer absent-value treatment ("not advertised" instead of "—").
     *   - Calmer empty-state copy.
     *   - Italicized status verb ("Capturing" subtler than "Stopped").
     *
     * No new colors, no custom typography, no extra components.
     */
    import type { AppSnapshot } from './sample-data';
    import { INTERFACES } from './sample-data';

    let { snapshot, theme }: { snapshot: AppSnapshot; theme: 'light' | 'dark' } = $props();

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

    let statusText = $derived(snapshot.statusText);
    let statusItalic = $derived(snapshot.isCapturing);
</script>

<div class="surface-a" class:dark={theme === 'dark'}>
    <div class="window">
        {#if snapshot.privilegeBanner === 'macos-bpf'}
            <div class="banner">
                <div class="banner-msg">Packet capture requires BPF device access.</div>
                <button class="banner-btn">Install BPF Access</button>
            </div>
        {:else if snapshot.privilegeBanner === 'windows-npcap'}
            <div class="banner">
                <div class="banner-msg">Npcap is required for packet capture.</div>
                <a class="banner-btn" href="#">Download Npcap</a>
                <div class="banner-hint">Enable "Allow non-administrators to capture" during install.</div>
            </div>
        {/if}

        <section class="card">
            <div class="row">
                <label for="a-iface">Interface</label>
                <div class="iface-row">
                    <select id="a-iface" disabled={snapshot.isCapturing} value={snapshot.selectedInterface}>
                        {#each INTERFACES as iface}
                            {@const compact = compactAddresses(iface.addresses)}
                            <option value={iface.name}>{iface.description}{compact ? ` (${compact})` : ''}</option>
                        {/each}
                    </select>
                    <button class="iface-refresh" disabled={snapshot.isCapturing} aria-label="Refresh">↻</button>
                </div>
            </div>

            <label class="switch-row">
                <span>Only show interfaces with an IP</span>
                <span class="switch" role="switch" aria-checked="true">
                    <span class="switch-thumb"></span>
                </span>
            </label>

            <div class="row">
                <label for="a-protocol">Protocol</label>
                <select id="a-protocol" disabled={snapshot.isCapturing} value={snapshot.protocol}>
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

        <section class="card">
            {#if snapshot.result}
                {@const r = snapshot.result}
                {@const vlan = valueOrAbsent(r.nativeVlan)}
                {@const voice = valueOrAbsent(r.voiceVlan)}
                {@const mtu = valueOrAbsent(r.mtu)}
                <dl class="result">
                    <dt>Switch</dt><dd>{r.switchName}</dd>
                    <dt>IP</dt><dd>{r.switchIp}</dd>
                    <dt>Port</dt><dd>{r.switchPort}</dd>
                    <dt>VLAN</dt><dd class:absent={vlan.absent}>{vlan.text}</dd>
                    <dt>Voice VLAN</dt><dd class:absent={voice.absent}>{voice.text}</dd>
                    <dt>MTU</dt><dd class:absent={mtu.absent}>{mtu.text}</dd>
                    <dt>Model</dt><dd>{r.switchModel}</dd>
                </dl>
            {:else}
                <p class="empty">Run a capture to see what switch you're on.</p>
            {/if}
        </section>

        <div class="status" class:error={snapshot.statusError} class:italic={statusItalic}>
            {statusText}
        </div>

        <div class="footer">
            <span class="version">v3.3.0-alpha.1</span>
        </div>
    </div>
</div>

<style>
    /* Direction A inherits the production app's per-OS approach.
       Pinned to macOS Tahoe values here so both comp panels render
       comparably in one browser instance. The values match what
       html[data-os="macos"] sets in style.css. */

    .surface-a {
        --bg-page: #ecebe8;
        --bg-card: rgba(255, 255, 255, 0.78);
        --card-border: 1px solid rgba(0, 0, 0, 0.06);
        --card-shadow: 0 1px 1px rgba(0, 0, 0, 0.03);
        --text: #1d1d1f;
        --text-muted: rgba(29, 29, 31, 0.62);
        --text-faint: rgba(29, 29, 31, 0.45);
        --accent: #0066cc;
        --switch-off: rgba(29, 29, 31, 0.22);
        --error: #c0392b;
        --highlight: rgba(0, 102, 204, 0.13);
        --highlight-text: #0a3a72;
        --highlight-border: rgba(0, 102, 204, 0.25);

        font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
        font-size: 13px;
        line-height: 1.4;
        color: var(--text);
        background: var(--bg-page);

        width: 400px;
        height: 500px;
        border-radius: 10px;
        overflow: hidden;
        position: relative;
        box-shadow:
            0 0 0 1px rgba(0, 0, 0, 0.08),
            0 24px 48px -12px rgba(0, 0, 0, 0.18),
            0 4px 8px rgba(0, 0, 0, 0.05);
    }

    .surface-a.dark {
        --bg-page: #1d1d1f;
        --bg-card: rgba(48, 48, 50, 0.85);
        --card-border: 1px solid rgba(255, 255, 255, 0.06);
        --card-shadow: none;
        --text: #f5f5f7;
        --text-muted: rgba(245, 245, 247, 0.62);
        --text-faint: rgba(245, 245, 247, 0.42);
        --accent: #4d97ff;
        --switch-off: rgba(245, 245, 247, 0.22);
        --error: #ff6b5b;
        --highlight: rgba(77, 151, 255, 0.18);
        --highlight-text: #d6e7ff;
        --highlight-border: rgba(77, 151, 255, 0.32);
    }

    .window {
        padding: 12px 12px 8px;
        height: 100%;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .card {
        background: var(--bg-card);
        border: var(--card-border);
        border-radius: 10px;
        box-shadow: var(--card-shadow);
        padding: 14px 18px;
        display: flex;
        flex-direction: column;
        gap: 13px;
    }

    .row {
        display: grid;
        grid-template-columns: 70px 1fr;
        column-gap: 10px;
        align-items: center;
        max-width: 320px;
        margin: 0 auto;
        width: 100%;
    }

    .row label {
        text-align: right;
        color: var(--text);
    }

    .row select,
    .iface-row select {
        width: 100%;
        font: inherit;
        padding: 2px 6px;
        background: rgba(255, 255, 255, 0.6);
        color: inherit;
        border: 1px solid rgba(0, 0, 0, 0.12);
        border-radius: 5px;
    }

    .surface-a.dark .row select,
    .surface-a.dark .iface-row select {
        background: rgba(255, 255, 255, 0.06);
        border-color: rgba(255, 255, 255, 0.12);
    }

    .iface-row {
        display: flex;
        gap: 6px;
        align-items: center;
    }

    .iface-row select {
        flex: 1;
        min-width: 0;
    }

    .iface-refresh {
        flex-shrink: 0;
        min-width: 26px;
        padding: 1px 6px;
        font: inherit;
        font-size: 14px;
        line-height: 1;
        background: rgba(255, 255, 255, 0.6);
        color: inherit;
        border: 1px solid rgba(0, 0, 0, 0.12);
        border-radius: 5px;
        cursor: pointer;
    }

    .surface-a.dark .iface-refresh {
        background: rgba(255, 255, 255, 0.06);
        border-color: rgba(255, 255, 255, 0.12);
    }

    .switch-row {
        display: flex;
        align-items: center;
        gap: 12px;
        max-width: 320px;
        margin: 0 auto;
        width: 100%;
        font-size: 0.92em;
        cursor: pointer;
        user-select: none;
    }

    .switch-row > span:first-child {
        flex: 1;
    }

    .switch {
        position: relative;
        width: 38px;
        height: 22px;
        border-radius: 11px;
        background: var(--accent);
        flex-shrink: 0;
        transition: background 200ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .switch-thumb {
        position: absolute;
        top: 2px;
        left: calc(100% - 20px);
        width: 18px;
        height: 18px;
        border-radius: 9px;
        background: #ffffff;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
        transition: left 200ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .progress {
        width: 100%;
        height: 3px;
        border-radius: 2px;
        background: rgba(0, 0, 0, 0.08);
        overflow: hidden;
        margin: 2px 0 0;
    }

    .surface-a.dark .progress {
        background: rgba(255, 255, 255, 0.1);
    }

    .progress-fill {
        width: 35%;
        height: 100%;
        background: var(--accent);
        border-radius: 2px;
        animation: a-progress 1.2s ease-in-out infinite;
    }

    @keyframes a-progress {
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
        gap: 12px;
        justify-content: center;
        margin-top: 2px;
    }

    .btn {
        font: inherit;
        padding: 4px 16px;
        min-width: 88px;
        background: rgba(255, 255, 255, 0.85);
        color: inherit;
        border: 1px solid rgba(0, 0, 0, 0.14);
        border-radius: 5px;
        cursor: pointer;
        box-shadow: 0 0.5px 0 rgba(255, 255, 255, 0.6) inset;
    }

    .surface-a.dark .btn {
        background: rgba(255, 255, 255, 0.08);
        border-color: rgba(255, 255, 255, 0.14);
        box-shadow: none;
    }

    .btn.primary {
        background: var(--accent);
        color: #ffffff;
        border-color: transparent;
        box-shadow: 0 1px 0 rgba(255, 255, 255, 0.18) inset;
    }

    .btn:disabled {
        opacity: 0.45;
        cursor: not-allowed;
    }

    .result {
        margin: 0;
        display: grid;
        grid-template-columns: 80px 1fr;
        column-gap: 10px;
        row-gap: 5px;
        align-items: baseline;
    }

    .result dt {
        text-align: right;
        color: var(--text-muted);
        font-size: 0.95em;
    }

    .result dd {
        margin: 0;
        font-weight: 500;
        overflow: hidden;
        text-overflow: ellipsis;
        font-variant-numeric: tabular-nums;
    }

    .result dd.absent {
        font-weight: 400;
        font-style: italic;
        color: var(--text-faint);
    }

    .empty {
        margin: 8px 0;
        text-align: center;
        color: var(--text-muted);
        font-size: 0.92em;
    }

    .status {
        text-align: center;
        color: var(--text-muted);
        font-size: 0.9em;
        min-height: 1.2em;
    }

    .status.italic {
        font-style: italic;
    }

    .status.error {
        color: var(--error);
        font-style: normal;
    }

    .footer {
        text-align: center;
    }

    .version {
        color: var(--text-faint);
        font-size: 0.8em;
        font-variant-numeric: tabular-nums;
    }

    .banner {
        background: var(--highlight);
        color: var(--highlight-text);
        border: 1px solid var(--highlight-border);
        border-radius: 6px;
        padding: 8px 12px;
        font-size: 0.92em;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .banner-btn {
        align-self: flex-start;
        font: inherit;
        padding: 3px 10px;
        background: rgba(255, 255, 255, 0.55);
        color: inherit;
        border: 1px solid var(--highlight-border);
        border-radius: 4px;
        text-decoration: none;
        cursor: pointer;
    }

    .surface-a.dark .banner-btn {
        background: rgba(255, 255, 255, 0.08);
    }

    .banner-hint {
        font-size: 0.86em;
        opacity: 0.78;
    }
</style>
