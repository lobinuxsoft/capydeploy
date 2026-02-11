export interface VersionInfo {
	version: string;
	commit: string;
	buildDate: string;
}

export interface AgentStatus {
	running: boolean;
	name: string;
	platform: string;
	version: string;
	port: number;
	ips: string[];
	acceptConnections: boolean;
	connectedHub?: ConnectedHub | null;
	telemetryEnabled: boolean;
	telemetryInterval: number;
}

export interface ConnectedHub {
	name: string;
	ip: string;
}

export interface SteamUserInfo {
	id: string;
	name: string;
}

export interface ShortcutInfo {
	appId: number;
	name: string;
	exe: string;
	startDir: string;
}

