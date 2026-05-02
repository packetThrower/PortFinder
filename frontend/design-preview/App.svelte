<script lang="ts">
    import DirectionA from './DirectionA.svelte';
    import DirectionB from './DirectionB.svelte';
    import { snapshotFor, type AppState } from './sample-data';

    let theme = $state<'light' | 'dark'>('light');
    let state = $state<AppState>('populated-full');

    // Mirror the theme onto <body> so the page background follows the
    // toggle even where the .shell grid doesn't cover (below the
    // footer, on short documents, during overscroll).
    $effect(() => {
        document.body.classList.toggle('theme-dark', theme === 'dark');
        document.body.classList.toggle('theme-light', theme === 'light');
    });

    const STATES: { key: AppState; label: string }[] = [
        { key: 'ready', label: 'Ready' },
        { key: 'capturing', label: 'Capturing' },
        { key: 'populated-full', label: 'Populated (full)' },
        { key: 'populated-partial', label: 'Populated (partial)' },
        { key: 'stopped', label: 'Stopped' },
        { key: 'error', label: 'Error' },
        { key: 'privilege-warning', label: 'BPF banner' },
        { key: 'no-pcap', label: 'No Npcap' },
    ];

    let snapshot = $derived(snapshotFor(state));
</script>

<div class="shell" class:shell-dark={theme === 'dark'}>
    <header class="bar">
        <div class="bar-title">
            <span class="bar-prod">PortFinder</span>
            <span class="bar-sub">design direction comparison</span>
        </div>

        <div class="bar-controls">
            <fieldset class="seg">
                <legend>Theme</legend>
                <label>
                    <input type="radio" bind:group={theme} value="light" />
                    <span>Light</span>
                </label>
                <label>
                    <input type="radio" bind:group={theme} value="dark" />
                    <span>Dark</span>
                </label>
            </fieldset>

            <fieldset class="seg seg-state">
                <legend>State</legend>
                {#each STATES as s}
                    <label>
                        <input type="radio" bind:group={state} value={s.key} />
                        <span>{s.label}</span>
                    </label>
                {/each}
            </fieldset>
        </div>
    </header>

    <main class="comp">
        <figure class="panel">
            <figcaption class="panel-cap">
                <span class="panel-tag panel-tag-a">A</span>
                <span class="panel-name">Native-refined</span>
                <span class="panel-note">macOS chrome, system colors, native widgets</span>
            </figcaption>
            <div class="panel-stage">
                <DirectionA {snapshot} {theme} />
            </div>
        </figure>

        <figure class="panel">
            <figcaption class="panel-cap">
                <span class="panel-tag panel-tag-b">B</span>
                <span class="panel-name">Custom-unified</span>
                <span class="panel-note">Linear-flavored, amber accent, Inter Variable</span>
            </figcaption>
            <div class="panel-stage">
                <DirectionB {snapshot} {theme} />
            </div>
        </figure>
    </main>

    <footer class="meta">
        <p>
            Both panels render the same <code>AppState</code>. Toggle theme + state above. Direction A inherits the production
            <code>Settings on Every Machine</code> approach; Direction B replaces it with a unified surface. Click any
            value in Direction B's result list to copy it.
        </p>
    </footer>
</div>

<style>
    :global(body) {
        margin: 0;
        background: oklch(94% 0.005 250);
        font-family: 'Inter', -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
        color: oklch(20% 0.01 250);
        -webkit-font-smoothing: antialiased;
    }

    :global(body.theme-dark) {
        background: oklch(15% 0.01 250);
        color: oklch(95% 0.005 250);
    }

    :global(html) {
        color-scheme: light dark;
    }

    .shell {
        min-height: 100vh;
        display: grid;
        grid-template-rows: auto 1fr auto;
    }

    .shell-dark {
        background: oklch(15% 0.01 250);
        color: oklch(95% 0.005 250);
    }

    :global(.shell-dark) {
        background: oklch(15% 0.01 250);
    }

    .bar {
        position: sticky;
        top: 0;
        z-index: 10;
        background: oklch(99% 0.003 250 / 0.92);
        backdrop-filter: blur(12px);
        -webkit-backdrop-filter: blur(12px);
        border-bottom: 1px solid oklch(91% 0.008 250);
        padding: 14px 28px;
        display: flex;
        gap: 32px;
        align-items: center;
        flex-wrap: wrap;
    }

    .shell-dark .bar {
        background: oklch(20% 0.012 250 / 0.92);
        border-bottom-color: oklch(28% 0.014 250);
    }

    .bar-title {
        display: flex;
        align-items: baseline;
        gap: 10px;
    }

    .bar-prod {
        font-size: 16px;
        font-weight: 600;
        letter-spacing: -0.01em;
    }

    .bar-sub {
        font-size: 13px;
        color: oklch(50% 0.012 250);
    }

    .shell-dark .bar-sub {
        color: oklch(70% 0.012 250);
    }

    .bar-controls {
        display: flex;
        gap: 24px;
        flex-wrap: wrap;
    }

    .seg {
        display: flex;
        gap: 6px;
        align-items: center;
        margin: 0;
        padding: 0;
        border: none;
        font-size: 12px;
    }

    .seg legend {
        float: left;
        margin-right: 8px;
        color: oklch(50% 0.012 250);
        text-transform: uppercase;
        letter-spacing: 0.06em;
        font-size: 10px;
        font-weight: 500;
    }

    .shell-dark .seg legend {
        color: oklch(60% 0.012 250);
    }

    .seg label {
        display: inline-flex;
        align-items: center;
        gap: 0;
        cursor: pointer;
    }

    .seg input[type="radio"] {
        position: absolute;
        opacity: 0;
        width: 1px;
        height: 1px;
        pointer-events: none;
    }

    .seg label span {
        padding: 4px 10px;
        border: 1px solid oklch(88% 0.008 250);
        font-weight: 500;
        background: oklch(99% 0.003 250);
        color: oklch(40% 0.012 250);
        transition:
            background 120ms cubic-bezier(0.25, 1, 0.5, 1),
            color 120ms cubic-bezier(0.25, 1, 0.5, 1);
    }

    .shell-dark .seg label span {
        background: oklch(22% 0.012 250);
        border-color: oklch(30% 0.014 250);
        color: oklch(70% 0.012 250);
    }

    .seg label:first-of-type span {
        border-radius: 5px 0 0 5px;
    }

    .seg label:last-of-type span {
        border-radius: 0 5px 5px 0;
    }

    .seg label:not(:first-of-type) span {
        border-left: none;
    }

    .seg label:hover span {
        background: oklch(96% 0.005 250);
        color: oklch(20% 0.012 250);
    }

    .shell-dark .seg label:hover span {
        background: oklch(26% 0.012 250);
        color: oklch(95% 0.005 250);
    }

    .seg input:checked + span {
        background: oklch(20% 0.015 250);
        color: oklch(98% 0.003 250);
        border-color: oklch(20% 0.015 250);
    }

    .shell-dark .seg input:checked + span {
        background: oklch(80% 0.13 75);
        color: oklch(15% 0.04 75);
        border-color: oklch(80% 0.13 75);
    }

    .seg input:focus-visible + span {
        outline: 2px solid oklch(72% 0.14 75);
        outline-offset: 1px;
    }

    .comp {
        display: grid;
        grid-template-columns: repeat(2, minmax(420px, 1fr));
        gap: 36px;
        padding: 36px 28px;
        align-items: start;
        justify-items: center;
    }

    @media (max-width: 920px) {
        .comp {
            grid-template-columns: 1fr;
        }
    }

    .panel {
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .panel-cap {
        display: flex;
        align-items: baseline;
        gap: 10px;
        font-size: 13px;
        max-width: 400px;
    }

    .panel-tag {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 22px;
        height: 22px;
        border-radius: 4px;
        font-size: 11px;
        font-weight: 600;
        font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace;
        background: oklch(94% 0.005 250);
        color: oklch(40% 0.012 250);
        flex-shrink: 0;
    }

    .shell-dark .panel-tag {
        background: oklch(26% 0.012 250);
        color: oklch(80% 0.012 250);
    }

    .panel-tag-b {
        background: oklch(86% 0.13 75);
        color: oklch(28% 0.08 70);
    }

    .panel-name {
        font-weight: 600;
    }

    .panel-note {
        color: oklch(50% 0.012 250);
        font-size: 12px;
    }

    .shell-dark .panel-note {
        color: oklch(65% 0.012 250);
    }

    .panel-stage {
        display: grid;
        place-items: center;
    }

    .meta {
        padding: 24px 28px 36px;
        text-align: center;
        max-width: 720px;
        margin: 0 auto;
        font-size: 13px;
        line-height: 1.55;
        color: oklch(50% 0.012 250);
    }

    .shell-dark .meta {
        color: oklch(65% 0.012 250);
    }

    .meta code {
        font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace;
        font-size: 12px;
        padding: 1px 5px;
        background: oklch(94% 0.005 250);
        border-radius: 3px;
    }

    .shell-dark .meta code {
        background: oklch(26% 0.012 250);
    }
</style>
