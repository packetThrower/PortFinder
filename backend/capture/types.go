package capture

type InterfaceInfo struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Addresses   string `json:"addresses"`
	HasIP       bool   `json:"hasIP"`
}

type CaptureRequest struct {
	InterfaceName string `json:"interfaceName"`
	Protocol      string `json:"protocol"`
}

type CaptureResult struct {
	SwitchName  string `json:"switchName"`
	SwitchIP    string `json:"switchIP"`
	SwitchPort  string `json:"switchPort"`
	NativeVLAN  string `json:"nativeVLAN"`
	VoiceVLAN   string `json:"voiceVLAN"`
	SwitchModel string `json:"switchModel"`
}
