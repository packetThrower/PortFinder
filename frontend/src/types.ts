export interface InterfaceInfo {
    name: string;
    description: string;
    addresses: string;
    hasIP: boolean;
}

export interface CaptureRequest {
    interfaceName: string;
    protocol: string;
}

export interface CaptureResult {
    switchName: string;
    switchIP: string;
    switchPort: string;
    nativeVLAN: string;
    voiceVLAN: string;
    switchModel: string;
}

export interface PrivilegeStatus {
    hasAccess: boolean;
    helperInstalled: boolean;
    inBPFGroup: boolean;
    canInstall: boolean;
    platform: string;
    npcapInstalled: boolean;
    npcapNonAdmin: boolean;
}
