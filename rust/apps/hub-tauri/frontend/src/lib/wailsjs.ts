// Tauri adapter — replaces Wails bindings with Tauri invoke/listen.
// This is the ONLY file that changed from the original Svelte frontend.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ---------------------------------------------------------------------------
// Runtime events (Wails-compatible wrapper)
// ---------------------------------------------------------------------------

export function EventsOn(name: string, cb: (...data: any[]) => void): () => void {
	let unlisten: UnlistenFn | null = null;
	listen(name, (event) => cb(event.payload)).then((fn) => {
		unlisten = fn;
	});
	return () => {
		if (unlisten) unlisten();
	};
}

export function EventsOff(_name: string): void {
	// No-op — Tauri uses the unlisten function returned by EventsOn.
}

// ---------------------------------------------------------------------------
// Connection commands
// ---------------------------------------------------------------------------

export const GetDiscoveredAgents = () => invoke('get_discovered_agents');
export const RefreshDiscovery = () => invoke('refresh_discovery');
export const ConnectAgent = (agentID: string) => invoke('connect_agent', { agentId: agentID });
export const DisconnectAgent = () => invoke('disconnect_agent');
export const GetConnectionStatus = () => invoke('get_connection_status');
export const GetAgentInstallPath = () => invoke('get_game_log_directory');

// ---------------------------------------------------------------------------
// Console log commands
// ---------------------------------------------------------------------------

export const SetConsoleLogFilter = (levelMask: number) =>
	invoke('set_console_log_filter', { levelMask });
export const SetConsoleLogEnabled = (enabled: boolean) =>
	invoke('set_console_log_enabled', { enabled });

// ---------------------------------------------------------------------------
// Game log wrapper
// ---------------------------------------------------------------------------

export const SetGameLogWrapper = (appID: number, enabled: boolean) =>
	invoke('set_game_log_wrapper', { appId: appID, enabled });

// ---------------------------------------------------------------------------
// Game log directory
// ---------------------------------------------------------------------------

export const GetGameLogDirectory = () => invoke('get_game_log_directory');
export const SetGameLogDirectory = (path: string) =>
	invoke('set_game_log_directory', { path });

// ---------------------------------------------------------------------------
// Pairing commands
// ---------------------------------------------------------------------------

export const ConfirmPairing = (pin: string) =>
	invoke('confirm_pairing', { agentId: (window as any).__pairingAgentId || '', code: pin });
export const CancelPairing = () => invoke('cancel_pairing');

// ---------------------------------------------------------------------------
// Game setup commands
// ---------------------------------------------------------------------------

export const GetGameSetups = () => invoke('get_game_setups');
export const AddGameSetup = (setup: any) => invoke('add_game_setup', { setup });
export const UpdateGameSetup = (id: string, setup: any) =>
	invoke('update_game_setup', { id, setup });
export const RemoveGameSetup = (id: string) => invoke('remove_game_setup', { id });
export const SelectFolder = () => invoke('select_folder');
export const UploadGame = (id: string) => invoke('upload_game', { id });

// ---------------------------------------------------------------------------
// Installed games commands
// ---------------------------------------------------------------------------

export const GetInstalledGames = (agentID: string) =>
	invoke('get_installed_games', { agentId: agentID });
export const DeleteGame = (agentID: string, appID: number) =>
	invoke('delete_game', { agentId: agentID, appId: appID });
export const UpdateGameArtwork = (
	appID: number,
	grid: string,
	hero: string,
	logo: string,
	icon: string,
	gameID: number
) => invoke('update_game_artwork', { appId: appID, grid, hero, logo, icon, gameId: gameID });

// ---------------------------------------------------------------------------
// Version / Hub info
// ---------------------------------------------------------------------------

export const GetVersion = () => invoke('get_version');
export const GetHubInfo = () => invoke('get_hub_info');
export const GetHubName = () => invoke('get_hub_name');
export const SetHubName = (name: string) => invoke('set_hub_name', { name });

// ---------------------------------------------------------------------------
// Settings / Cache
// ---------------------------------------------------------------------------

export const GetSteamGridDBAPIKey = () => invoke('get_steamgriddb_api_key');
export const SetSteamGridDBAPIKey = (key: string) =>
	invoke('set_steamgriddb_api_key', { key });
export const GetCacheSize = () => invoke('get_cache_size');
export const ClearImageCache = () => invoke('clear_image_cache');
export const OpenCacheFolder = () => invoke('open_cache_folder');
export const GetImageCacheEnabled = () => invoke('get_image_cache_enabled');
export const SetImageCacheEnabled = (enabled: boolean) =>
	invoke('set_image_cache_enabled', { enabled });

// ---------------------------------------------------------------------------
// Artwork file selection
// ---------------------------------------------------------------------------

export const SelectArtworkFile = () => invoke('select_artwork_file');
export const GetArtworkPreview = (url: string) => invoke('get_artwork_preview', { url });

// ---------------------------------------------------------------------------
// SteamGridDB commands
// ---------------------------------------------------------------------------

export const SearchGames = (query: string) => invoke('search_games', { query });
export const GetGrids = (gameID: number, filters: any, page: number) =>
	invoke('get_grids', { gameId: gameID, filters, page });
export const GetHeroes = (gameID: number, filters: any, page: number) =>
	invoke('get_heroes', { gameId: gameID, filters, page });
export const GetLogos = (gameID: number, filters: any, page: number) =>
	invoke('get_logos', { gameId: gameID, filters, page });
export const GetIcons = (gameID: number, filters: any, page: number) =>
	invoke('get_icons', { gameId: gameID, filters, page });
