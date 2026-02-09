export namespace main {
	
	export class ConnectedHub {
	    id: string;
	    name: string;
	    ip: string;
	
	    static createFrom(source: any = {}) {
	        return new ConnectedHub(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.ip = source["ip"];
	    }
	}
	export class AgentStatus {
	    running: boolean;
	    name: string;
	    platform: string;
	    version: string;
	    port: number;
	    ips: string[];
	    acceptConnections: boolean;
	    connectedHub?: ConnectedHub;
	
	    static createFrom(source: any = {}) {
	        return new AgentStatus(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.running = source["running"];
	        this.name = source["name"];
	        this.platform = source["platform"];
	        this.version = source["version"];
	        this.port = source["port"];
	        this.ips = source["ips"];
	        this.acceptConnections = source["acceptConnections"];
	        this.connectedHub = this.convertValues(source["connectedHub"], ConnectedHub);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class AuthorizedHubInfo {
	    id: string;
	    name: string;
	    pairedAt: string;
	    lastSeen: string;
	
	    static createFrom(source: any = {}) {
	        return new AuthorizedHubInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.pairedAt = source["pairedAt"];
	        this.lastSeen = source["lastSeen"];
	    }
	}
	export class ShortcutInfo {
	    appId: number;
	    name: string;
	    exe: string;
	    startDir: string;
	
	    static createFrom(source: any = {}) {
	        return new ShortcutInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.appId = source["appId"];
	        this.name = source["name"];
	        this.exe = source["exe"];
	        this.startDir = source["startDir"];
	    }
	}
	export class SteamUserInfo {
	    id: string;
	    name: string;
	
	    static createFrom(source: any = {}) {
	        return new SteamUserInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	    }
	}
	export class VersionInfo {
	    version: string;
	    commit: string;
	    buildDate: string;
	
	    static createFrom(source: any = {}) {
	        return new VersionInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.version = source["version"];
	        this.commit = source["commit"];
	        this.buildDate = source["buildDate"];
	    }
	}

}

