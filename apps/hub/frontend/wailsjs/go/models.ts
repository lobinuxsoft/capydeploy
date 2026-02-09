export namespace config {
	
	export class GameSetup {
	    id: string;
	    name: string;
	    local_path: string;
	    executable: string;
	    launch_options?: string;
	    tags?: string;
	    install_path: string;
	    griddb_game_id?: number;
	    grid_portrait?: string;
	    grid_landscape?: string;
	    hero_image?: string;
	    logo_image?: string;
	    icon_image?: string;
	
	    static createFrom(source: any = {}) {
	        return new GameSetup(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.local_path = source["local_path"];
	        this.executable = source["executable"];
	        this.launch_options = source["launch_options"];
	        this.tags = source["tags"];
	        this.install_path = source["install_path"];
	        this.griddb_game_id = source["griddb_game_id"];
	        this.grid_portrait = source["grid_portrait"];
	        this.grid_landscape = source["grid_landscape"];
	        this.hero_image = source["hero_image"];
	        this.logo_image = source["logo_image"];
	        this.icon_image = source["icon_image"];
	    }
	}

}

export namespace main {
	
	export class ArtworkFileResult {
	    path: string;
	    dataURI: string;
	    contentType: string;
	    size: number;
	
	    static createFrom(source: any = {}) {
	        return new ArtworkFileResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.dataURI = source["dataURI"];
	        this.contentType = source["contentType"];
	        this.size = source["size"];
	    }
	}
	export class ConnectionStatus {
	    connected: boolean;
	    agentId: string;
	    agentName: string;
	    platform: string;
	    host: string;
	    port: number;
	    ips: string[];
	    supportedImageFormats: string[];

	    static createFrom(source: any = {}) {
	        return new ConnectionStatus(source);
	    }

	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.connected = source["connected"];
	        this.agentId = source["agentId"];
	        this.agentName = source["agentName"];
	        this.platform = source["platform"];
	        this.host = source["host"];
	        this.port = source["port"];
	        this.ips = source["ips"];
	        this.supportedImageFormats = source["supportedImageFormats"];
	    }
	}
	export class DiscoveredAgentInfo {
	    id: string;
	    name: string;
	    platform: string;
	    version: string;
	    host: string;
	    port: number;
	    ips: string[];
	    discoveredAt: string;
	    lastSeen: string;
	    online: boolean;
	
	    static createFrom(source: any = {}) {
	        return new DiscoveredAgentInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.platform = source["platform"];
	        this.version = source["version"];
	        this.host = source["host"];
	        this.port = source["port"];
	        this.ips = source["ips"];
	        this.discoveredAt = source["discoveredAt"];
	        this.lastSeen = source["lastSeen"];
	        this.online = source["online"];
	    }
	}
	export class HubInfo {
	    id: string;
	    name: string;
	    platform: string;
	
	    static createFrom(source: any = {}) {
	        return new HubInfo(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.platform = source["platform"];
	    }
	}
	export class InstalledGame {
	    name: string;
	    path: string;
	    size: string;
	    appId?: number;
	
	    static createFrom(source: any = {}) {
	        return new InstalledGame(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.size = source["size"];
	        this.appId = source["appId"];
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

export namespace steamgriddb {
	
	export class GridData {
	    id: number;
	    score: number;
	    style: string;
	    width: number;
	    height: number;
	    nsfw: boolean;
	    humor: boolean;
	    mime: string;
	    language: string;
	    url: string;
	    thumb: string;
	    lock: boolean;
	    epilepsy: boolean;
	    upvotes: number;
	    downvotes: number;
	
	    static createFrom(source: any = {}) {
	        return new GridData(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.score = source["score"];
	        this.style = source["style"];
	        this.width = source["width"];
	        this.height = source["height"];
	        this.nsfw = source["nsfw"];
	        this.humor = source["humor"];
	        this.mime = source["mime"];
	        this.language = source["language"];
	        this.url = source["url"];
	        this.thumb = source["thumb"];
	        this.lock = source["lock"];
	        this.epilepsy = source["epilepsy"];
	        this.upvotes = source["upvotes"];
	        this.downvotes = source["downvotes"];
	    }
	}
	export class ImageData {
	    id: number;
	    score: number;
	    style: string;
	    width: number;
	    height: number;
	    nsfw: boolean;
	    humor: boolean;
	    mime: string;
	    language: string;
	    url: string;
	    thumb: string;
	    lock: boolean;
	    epilepsy: boolean;
	    upvotes: number;
	    downvotes: number;
	
	    static createFrom(source: any = {}) {
	        return new ImageData(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.score = source["score"];
	        this.style = source["style"];
	        this.width = source["width"];
	        this.height = source["height"];
	        this.nsfw = source["nsfw"];
	        this.humor = source["humor"];
	        this.mime = source["mime"];
	        this.language = source["language"];
	        this.url = source["url"];
	        this.thumb = source["thumb"];
	        this.lock = source["lock"];
	        this.epilepsy = source["epilepsy"];
	        this.upvotes = source["upvotes"];
	        this.downvotes = source["downvotes"];
	    }
	}
	export class ImageFilters {
	    style: string;
	    mimeType: string;
	    imageType: string;
	    dimension: string;
	    showNsfw: boolean;
	    showHumor: boolean;
	
	    static createFrom(source: any = {}) {
	        return new ImageFilters(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.style = source["style"];
	        this.mimeType = source["mimeType"];
	        this.imageType = source["imageType"];
	        this.dimension = source["dimension"];
	        this.showNsfw = source["showNsfw"];
	        this.showHumor = source["showHumor"];
	    }
	}
	export class SearchResult {
	    id: number;
	    name: string;
	    types: string[];
	    verified: boolean;
	
	    static createFrom(source: any = {}) {
	        return new SearchResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.types = source["types"];
	        this.verified = source["verified"];
	    }
	}

}

