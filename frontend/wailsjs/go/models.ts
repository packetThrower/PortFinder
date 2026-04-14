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
	    hasIP: boolean;

	    static createFrom(source: any = {}) {
	        return new InterfaceInfo(source);
	    }

	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.description = source["description"];
	        this.addresses = source["addresses"];
	        this.hasIP = source["hasIP"];
	    }
	}

}

export namespace privilege {
	
	export class PrivilegeStatus {
	    hasAccess: boolean;
	    helperInstalled: boolean;
	    inBPFGroup: boolean;
	    canInstall: boolean;
	    platform: string;
	    npcapInstalled: boolean;
	    npcapNonAdmin: boolean;
	
	    static createFrom(source: any = {}) {
	        return new PrivilegeStatus(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.hasAccess = source["hasAccess"];
	        this.helperInstalled = source["helperInstalled"];
	        this.inBPFGroup = source["inBPFGroup"];
	        this.canInstall = source["canInstall"];
	        this.platform = source["platform"];
	        this.npcapInstalled = source["npcapInstalled"];
	        this.npcapNonAdmin = source["npcapNonAdmin"];
	    }
	}

}

