export interface InterfaceInfo {
    name: string;
    description: string;
    addresses: string;
    hasIp: boolean;
}

export interface CaptureRequest {
    interfaceName: string;
    protocol: string;
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

export interface PrivilegeStatus {
    hasAccess: boolean;
    helperInstalled: boolean;
    inBpfGroup: boolean;
    canInstall: boolean;
    platform: string;
    npcapInstalled: boolean;
    npcapNonAdmin: boolean;
}
