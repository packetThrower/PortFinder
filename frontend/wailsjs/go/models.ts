export namespace capture {
	
	export class CaptureRequest {
	    interfaceName: string;
	    protocol: string;
	
	    static createFrom(source: any = {}) {
	        return new CaptureRequest(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.interfaceName = source["interfaceName"];
	        this.protocol = source["protocol"];
	    }
	}
	export class CaptureResult {
	    switchName: string;
	    switchIP: string;
	    switchPort: string;
	    nativeVLAN: string;
	    voiceVLAN: string;
	    switchModel: string;
	
	    static createFrom(source: any = {}) {
	        return new CaptureResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.switchName = source["switchName"];
	        this.switchIP = source["switchIP"];
	        this.switchPort = source["switchPort"];
	        this.nativeVLAN = source["nativeVLAN"];
	        this.voiceVLAN = source["voiceVLAN"];
	        this.switchModel = source["switchModel"];
	    }
	}
	export class InterfaceInfo {
	    name: string;
	    description: string;
	    addresses: string;
	
	    static createFrom(source: any = {}) {
	        return new InterfaceInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.description = source["description"];
	        this.addresses = source["addresses"];
	    }
	}

}

