import { useState, useEffect } from 'react';
import './App.css';
import { GetInterfaces, StartCapture, StopCapture, GetVersion, CheckPrivileges } from '../wailsjs/go/main/App';

interface InterfaceInfo {
    name: string;
    description: string;
    addresses: string;
}

interface CaptureResult {
    switchName: string;
    switchIP: string;
    switchPort: string;
    nativeVLAN: string;
    voiceVLAN: string;
    switchModel: string;
}

function App() {
    const [interfaces, setInterfaces] = useState<InterfaceInfo[]>([]);
    const [selectedInterface, setSelectedInterface] = useState('');
    const [protocol, setProtocol] = useState<'CDP' | 'LLDP'>('CDP');
    const [isCapturing, setIsCapturing] = useState(false);
    const [result, setResult] = useState<CaptureResult | null>(null);
    const [status, setStatus] = useState('Ready');
    const [error, setError] = useState('');
    const [hasPrivileges, setHasPrivileges] = useState(true);
    const [version, setVersion] = useState('');

    useEffect(() => {
        GetInterfaces().then((ifaces) => {
            setInterfaces(ifaces || []);
        }).catch((err) => {
            setError('Failed to load interfaces: ' + err);
        });

        CheckPrivileges().then(setHasPrivileges);
        GetVersion().then(setVersion);
    }, []);

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

    return (
        <div className="app">
            {!hasPrivileges && (
                <div className="privilege-warning">
                    Elevated privileges required for packet capture.
                    Run with sudo (Linux/macOS) or as Administrator (Windows).
                </div>
            )}

            <div className="form-group">
                <label>Select a NIC:</label>
                <select
                    value={selectedInterface}
                    onChange={(e) => setSelectedInterface(e.target.value)}
                    disabled={isCapturing}
                >
                    {interfaces.map((iface) => (
                        <option key={iface.name || '__all__'} value={iface.name}>
                            {iface.description || iface.name || 'Sniff all Interfaces'}
                            {iface.addresses ? ` (${iface.addresses})` : ''}
                        </option>
                    ))}
                </select>
            </div>

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
