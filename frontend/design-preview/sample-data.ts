// Realistic sample data for design-preview comps. Lifted from the
// shapes that real CDP / LLDP / MNDP packets produce in the field.
// Mirrors the bindings.ts types but doesn't import them to keep the
// preview standalone (Tauri runtime not loaded here).

export interface Interface {
    name: string;
    description: string;
    addresses: string;
    hasIp: boolean;
}

export interface CaptureResult {
    switchName: string;
    switchIp: string;
    switchPort: string;
    nativeVlan: string;
    voiceVlan: string;
    mtu: string;
    switchModel: string;
}

export const INTERFACES: Interface[] = [
    { name: '', description: 'Sniff all Interfaces', addresses: '', hasIp: false },
    { name: 'en0', description: 'Wi-Fi', addresses: '10.42.18.74, fe80::1c45:8a93:5fa1:b4d2', hasIp: true },
    { name: 'en1', description: 'Thunderbolt Ethernet', addresses: '10.42.0.124', hasIp: true },
    { name: 'utun0', description: 'Tailscale', addresses: '100.94.211.7', hasIp: true },
    { name: 'awdl0', description: 'AWDL', addresses: 'fe80::a8b1:7eff:fe43:9c20', hasIp: false },
];

// "Hero" capture — the one populated state field techs see most.
export const CAPTURE_FULL: CaptureResult = {
    switchName: 'core-sw-01.lab.example.com',
    switchIp: '10.42.0.1',
    switchPort: 'GigabitEthernet1/0/24',
    nativeVlan: '100',
    voiceVlan: '200',
    mtu: '1500',
    switchModel: 'Cisco IOS Software, C9300 Software (cat9k_iosxe)',
};

// Switch that doesn't advertise voice VLAN or MTU. Common on
// non-Cisco gear and on switches with LLDP-MED disabled. The
// "honest about absence" PRODUCT.md principle decides how each
// direction renders these.
export const CAPTURE_PARTIAL: CaptureResult = {
    switchName: 'edge-2.colo.example.net',
    switchIp: '198.51.100.42',
    switchPort: 'ether3',
    nativeVlan: '1',
    voiceVlan: 'N/A',
    mtu: 'N/A',
    switchModel: 'MikroTik RB5009UG+S+',
};

// Capture state machine for the comparison shell.
export type AppState =
    | 'ready'
    | 'capturing'
    | 'populated-full'
    | 'populated-partial'
    | 'stopped'
    | 'error'
    | 'privilege-warning'
    | 'no-pcap';

export interface AppSnapshot {
    state: AppState;
    statusText: string;
    statusError: boolean;
    result: CaptureResult | null;
    isCapturing: boolean;
    privilegeBanner: 'none' | 'macos-bpf' | 'windows-npcap';
    selectedInterface: string;
    protocol: 'LLDP' | 'CDP' | 'MNDP';
}

export function snapshotFor(state: AppState): AppSnapshot {
    const base: AppSnapshot = {
        state,
        statusText: 'Ready',
        statusError: false,
        result: null,
        isCapturing: false,
        privilegeBanner: 'none',
        selectedInterface: 'en1',
        protocol: 'LLDP',
    };
    switch (state) {
        case 'ready':
            return base;
        case 'capturing':
            return { ...base, statusText: 'Capturing LLDP packets...', isCapturing: true };
        case 'populated-full':
            return { ...base, statusText: 'Capture complete', result: CAPTURE_FULL };
        case 'populated-partial':
            return { ...base, statusText: 'Capture complete', result: CAPTURE_PARTIAL };
        case 'stopped':
            return { ...base, statusText: 'Capture stopped' };
        case 'error':
            return {
                ...base,
                statusText: 'No packet captured from any interface',
                statusError: true,
            };
        case 'privilege-warning':
            return { ...base, privilegeBanner: 'macos-bpf' };
        case 'no-pcap':
            return {
                ...base,
                privilegeBanner: 'windows-npcap',
                selectedInterface: '',
            };
    }
}
