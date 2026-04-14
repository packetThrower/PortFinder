import { useState, useEffect } from 'react';
import './App.css';
import { GetInterfaces, StartCapture, StopCapture, GetVersion, GetPrivilegeStatus, InstallBPFHelper } from '../wailsjs/go/main/App';

interface InterfaceInfo {
    name: string;
    description: string;
    addresses: string;
    hasIP: boolean;
}

interface CaptureResult {
    switchName: string;
    switchIP: string;
    switchPort: string;
    nativeVLAN: string;
    voiceVLAN: string;
    switchModel: string;
}

interface PrivilegeStatus {
    hasAccess: boolean;
    helperInstalled: boolean;
    inBPFGroup: boolean;
    canInstall: boolean;
    platform: string;
    npcapInstalled: boolean;
    npcapNonAdmin: boolean;
}

function App() {
    const [interfaces, setInterfaces] = useState<InterfaceInfo[]>([]);
    const [selectedInterface, setSelectedInterface] = useState('');
    const [protocol, setProtocol] = useState<'CDP' | 'LLDP'>('LLDP');
    const [isCapturing, setIsCapturing] = useState(false);
    const [result, setResult] = useState<CaptureResult | null>(null);
    const [status, setStatus] = useState('Ready');
    const [error, setError] = useState('');
    const [privStatus, setPrivStatus] = useState<PrivilegeStatus | null>(null);
    const [isInstalling, setIsInstalling] = useState(false);
    const [version, setVersion] = useState('');
    const [showOnlyWithIPs, setShowOnlyWithIPs] = useState(true);

    const filteredInterfaces = showOnlyWithIPs
        ? interfaces.filter((iface) => iface.name === '' || iface.hasIP)
        : interfaces;

    const refreshPrivileges = () => {
        GetPrivilegeStatus().then(setPrivStatus);
    };

    useEffect(() => {
        GetInterfaces().then((ifaces) => {
            setInterfaces(ifaces || []);
        }).catch((err) => {
            setError('Failed to load interfaces: ' + err);
        });

        refreshPrivileges();
        GetVersion().then(setVersion);
    }, []);

    const handleInstallBPF = async () => {
        setIsInstalling(true);
        setError('');
        try {
            await InstallBPFHelper();
            setStatus('BPF access installed. You may need to restart the app.');
            refreshPrivileges();
        } catch (err: unknown) {
            const msg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'Installation failed');
            setError(msg);
        } finally {
            setIsInstalling(false);
        }
    };

    const handleStart = async () => {
        setIsCapturing(true);
        setError('');
        setResult(null);
        setStatus('Capturing ' + protocol + ' packets...');

        try {
            const res = await StartCapture({
                interfaceName: selectedInterface,
                protocol: protocol,
            });
            if (res) {
                setResult(res);
                setStatus('Capture complete');
            }
        } catch (err: unknown) {
            const msg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'Capture failed');
            if (msg.includes('cancelled')) {
                setStatus('Capture stopped');
            } else {
                setError(msg);
                setStatus('Error');
            }
        } finally {
            setIsCapturing(false);
        }
    };

    const handleStop = () => {
        StopCapture();
        setStatus('Stopping...');
    };

    const renderPrivilegeWarning = () => {
        if (!privStatus || privStatus.hasAccess) return null;

        if (privStatus.platform === 'darwin' && privStatus.canInstall) {
            return (
                <div className="privilege-warning">
                    <div>Packet capture requires BPF device access.</div>
                    <button
                        className="install-btn"
                        onClick={handleInstallBPF}
                        disabled={isInstalling}
                    >
                        {isInstalling ? 'Installing...' : 'Install BPF Access'}
                    </button>
                </div>
            );
        }

        if (privStatus.platform === 'linux') {
            return (
                <div className="privilege-warning">
                    Run with sudo or install the .deb/.rpm package (sets CAP_NET_RAW).
                </div>
            );
        }

        if (privStatus.platform === 'windows') {
            if (!privStatus.npcapInstalled) {
                return (
                    <div className="privilege-warning">
                        <div>Npcap is required for packet capture.</div>
                        <a
                            className="install-btn"
                            href="https://npcap.com/#download"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            Download Npcap
                        </a>
                        <div style={{ marginTop: 6, fontSize: 12, opacity: 0.8 }}>
                            Enable "Allow non-administrators to capture" during install.
                        </div>
                    </div>
                );
            }
            if (!privStatus.npcapNonAdmin) {
                return (
                    <div className="privilege-warning">
                        <div>Npcap is installed but requires admin privileges.</div>
                        <div style={{ marginTop: 6, fontSize: 12, opacity: 0.8 }}>
                            Reinstall Npcap with "Allow non-administrators to capture" enabled, or run as Administrator.
                        </div>
                    </div>
                );
            }
        }

        return (
            <div className="privilege-warning">
                Elevated privileges required for packet capture.
            </div>
        );
    };

    return (
        <div className="app">
            {renderPrivilegeWarning()}

            <div className="form-group">
                <label>Select a NIC:</label>
                <select
                    value={selectedInterface}
                    onChange={(e) => setSelectedInterface(e.target.value)}
                    disabled={isCapturing}
                >
                    {filteredInterfaces.map((iface) => (
                        <option key={iface.name || '__all__'} value={iface.name}>
                            {iface.description || iface.name || 'Sniff all Interfaces'}
                            {iface.addresses ? ` (${iface.addresses})` : ''}
                        </option>
                    ))}
                </select>
            </div>

            <label className="checkbox-label">
                <input
                    type="checkbox"
                    checked={showOnlyWithIPs}
                    onChange={(e) => setShowOnlyWithIPs(e.target.checked)}
                    disabled={isCapturing}
                />
                Show only interfaces with IPs
            </label>

            <div className="form-group">
                <label>Switch:</label>
                <input type="text" readOnly value={result?.switchName || ''} />
            </div>

            <div className="form-group">
                <label>Switch IP:</label>
                <input type="text" readOnly value={result?.switchIP || ''} />
            </div>

            <div className="form-group">
                <label>Switchport:</label>
                <input type="text" readOnly value={result?.switchPort || ''} />
            </div>

            <div className="form-group">
                <label>VLAN:</label>
                <input type="text" readOnly value={result?.nativeVLAN || ''} />
            </div>

            <div className="form-group">
                <label>Voice VLAN:</label>
                <input type="text" readOnly value={result?.voiceVLAN || ''} />
            </div>

            <div className="form-group">
                <label>Switch Model:</label>
                <input type="text" readOnly value={result?.switchModel || ''} />
            </div>

            {isCapturing && (
                <div className="progress-bar">
                    <div className="progress-fill" />
                </div>
            )}

            <div className="button-row">
                <button onClick={handleStart} disabled={isCapturing}>
                    Start
                </button>
                <button
                    className="stop"
                    onClick={handleStop}
                    disabled={!isCapturing}
                >
                    Stop
                </button>
            </div>

            <div className="form-group">
                <label>Protocol:</label>
                <div className="protocol-selector">
                    <label>
                        <input
                            type="radio"
                            name="protocol"
                            value="CDP"
                            checked={protocol === 'CDP'}
                            onChange={() => setProtocol('CDP')}
                            disabled={isCapturing}
                        />
                        CDP
                    </label>
                    <label>
                        <input
                            type="radio"
                            name="protocol"
                            value="LLDP"
                            checked={protocol === 'LLDP'}
                            onChange={() => setProtocol('LLDP')}
                            disabled={isCapturing}
                        />
                        LLDP
                    </label>
                </div>
            </div>

            <div className={`status-text ${error ? 'error-text' : ''}`}>
                {error || status}
            </div>

            {version && (
                <div className="version-text">v{version}</div>
            )}
        </div>
    );
}

export default App;
