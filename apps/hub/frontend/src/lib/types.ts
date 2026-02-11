export interface VersionInfo {
	version: string;
	commit: string;
	buildDate: string;
}

export interface HubInfo {
	id: string;
	name: string;
	platform: string;
}

// Agent types (replaces Device types)
export interface DiscoveredAgent {
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
}

export interface ConnectionStatus {
	connected: boolean;
	agentId: string;
	agentName: string;
	platform: string;
	host: string;
	port: number;
	ips: string[];
	supportedImageFormats: string[];
}

// Game setup types
export interface GameSetup {
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
}

export interface InstalledGame {
	name: string;
	path: string;
	size: string;
	appId?: number;
}

export interface UploadProgress {
	progress: number;
	status: string;
	error?: string;
	done: boolean;
}

// Telemetry types
export interface TelemetryStatus {
	enabled: boolean;
	interval: number;
}

export interface TelemetryData {
	timestamp: number;
	cpu?: CPUMetrics;
	gpu?: GPUMetrics;
	memory?: MemoryMetrics;
	battery?: BatteryMetrics;
	power?: PowerMetrics;
	fan?: FanMetrics;
	steam?: SteamStatusData;
}

export interface CPUMetrics {
	usagePercent: number;
	tempCelsius: number;
	freqMHz: number;
}

export interface GPUMetrics {
	usagePercent: number;
	tempCelsius: number;
	freqMHz: number;
	memFreqMHz?: number;
	vramUsedBytes?: number;
	vramTotalBytes?: number;
}

export interface MemoryMetrics {
	totalBytes: number;
	availableBytes: number;
	usagePercent: number;
	swapTotalBytes?: number;
	swapFreeBytes?: number;
}

export interface BatteryMetrics {
	capacity: number;
	status: string;
}

export interface PowerMetrics {
	tdpWatts: number;
	powerWatts: number;
}

export interface FanMetrics {
	rpm: number;
}

export interface SteamStatusData {
	running: boolean;
	gamingMode: boolean;
}

// Console log types
export interface ConsoleLogStatus {
	enabled: boolean;
}

export interface StyledSegment {
	text: string;
	css?: string;
}

export interface ConsoleLogEntry {
	timestamp: number;
	level: string;
	source: string;
	text: string;
	url?: string;
	line?: number;
	segments?: StyledSegment[];
}

export interface ConsoleLogBatch {
	entries: ConsoleLogEntry[];
	dropped: number;
}

// SteamGridDB types
export interface SearchResult {
	id: number;
	name: string;
	types: string[];
	verified: boolean;
}

export interface GridData {
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
}

export interface ImageData {
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
}

export interface ImageFilters {
	style: string;
	mimeType: string;
	imageType: string;
	dimension: string;
	showNsfw: boolean;
	showHumor: boolean;
}

export interface ArtworkFileResult {
	path: string;
	dataURI: string;
	contentType: string;
	size: number;
}

export interface ArtworkSelection {
	gridDBGameID: number;
	gridPortrait: string;
	gridLandscape: string;
	heroImage: string;
	logoImage: string;
	iconImage: string;
}

// Filter options
export const gridStyles = ['All Styles', 'alternate', 'white_logo', 'no_logo', 'blurred', 'material'];
export const heroStyles = ['All Styles', 'alternate', 'blurred', 'material'];
export const logoStyles = ['All Styles', 'official', 'white', 'black', 'custom'];
export const iconStyles = ['All Styles', 'official', 'custom'];

export const capsuleDimensions = ['All Sizes', '600x900', '342x482', '660x930', '512x512', '1024x1024'];
export const wideCapsuleDimensions = ['All Sizes', '460x215', '920x430'];
export const heroDimensions = ['All Sizes', '1920x620', '3840x1240', '1600x650'];
export const logoDimensions = ['All Sizes'];
export const iconDimensions = ['All Sizes', '512x512', '256x256', '128x128', '64x64', '32x32', '24x24', '16x16'];

export const gridMimes = ['All Formats', 'image/png', 'image/jpeg', 'image/webp'];
export const logoMimes = ['All Formats', 'image/png', 'image/webp'];
export const iconMimes = ['All Formats', 'image/png', 'image/vnd.microsoft.icon'];

export const animationOptions = ['All', 'Static Only', 'Animated Only'];
