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
// Status / Version
// ---------------------------------------------------------------------------

export const GetVersion = () => invoke('get_version');
export const GetStatus = () => invoke('get_status');

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export const SetName = (name: string) => invoke('set_name', { name });
export const GetInstallPath = () => invoke('get_install_path');
export const SetInstallPath = (path: string) => invoke('set_install_path', { path });
export const SelectInstallPath = () => invoke('select_install_path');

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

export const SetAcceptConnections = (accept: boolean) =>
	invoke('set_accept_connections', { accept });
export const DisconnectHub = () => invoke('disconnect_hub');

// ---------------------------------------------------------------------------
// Steam
// ---------------------------------------------------------------------------

export const GetSteamUsers = () => invoke('get_steam_users');
export const GetShortcuts = (userId: string) => invoke('get_shortcuts', { userId });
export const DeleteShortcut = (userId: string, appId: number) =>
	invoke('delete_shortcut', { userId, appId });

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

export const SetTelemetryEnabled = (enabled: boolean) =>
	invoke('set_telemetry_enabled', { enabled });
export const SetTelemetryInterval = (seconds: number) =>
	invoke('set_telemetry_interval', { seconds });

// ---------------------------------------------------------------------------
// Console log
// ---------------------------------------------------------------------------

export const SetConsoleLogEnabled = (enabled: boolean) =>
	invoke('set_console_log_enabled', { enabled });

// ---------------------------------------------------------------------------
// Authorized Hubs
// ---------------------------------------------------------------------------

export const GetAuthorizedHubs = () => invoke('get_authorized_hubs');
export const RevokeHub = (hubId: string) => invoke('revoke_hub', { hubId });
