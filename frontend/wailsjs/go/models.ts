export namespace config {
	
	export class DeviceConfig {
	    name: string;
	    host: string;
	    port: number;
	    user: string;
	    key_file?: string;
	    password?: string;
	
	    static createFrom(source: any = {}) {
	        return new DeviceConfig(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.host = source["host"];
	        this.port = source["port"];
	        this.user = source["user"];
	        this.key_file = source["key_file"];
	        this.password = source["password"];
	    }
	}
	export class GameSetup {
	    id: string;
	    name: string;
	    local_path: string;
	    executable: string;
	    launch_options?: string;
	    tags?: string;
	    remote_path: string;
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
	        this.remote_path = source["remote_path"];
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
	
	export class ConnectionStatus {
	    connected: boolean;
	    deviceName: string;
	    host: string;
	    port: number;
	
	    static createFrom(source: any = {}) {
	        return new ConnectionStatus(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.connected = source["connected"];
	        this.deviceName = source["deviceName"];
	        this.host = source["host"];
	        this.port = source["port"];
	    }
	}
	export class InstalledGame {
	    name: string;
	    path: string;
	    size: string;
	
	    static createFrom(source: any = {}) {
	        return new InstalledGame(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.size = source["size"];
	    }
	}
	export class NetworkDevice {
	    ip: string;
	    hostname: string;
	    hasSSH: boolean;
	
	    static createFrom(source: any = {}) {
	        return new NetworkDevice(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.ip = source["ip"];
	        this.hostname = source["hostname"];
	        this.hasSSH = source["hasSSH"];
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

