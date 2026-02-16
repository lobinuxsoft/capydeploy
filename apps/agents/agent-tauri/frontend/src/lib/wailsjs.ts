// Tauri adapter — replaces Wails bindings with Tauri invoke/listen.
// This is the ONLY file that changed from the original Svelte frontend.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AgentStatus, VersionInfo, SteamUserInfo, ShortcutInfo } from '$lib/types';

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

// AuthorizedHub DTO shape (matches backend AuthorizedHubDto).
interface AuthorizedHubDto {
	id: string;
	name: string;
	pairedAt: string;
	lastSeen: string;
}

// ---------------------------------------------------------------------------
// Status / Version
// ---------------------------------------------------------------------------

export const GetVersion = () => invoke<VersionInfo>('get_version');
export const GetStatus = () => invoke<AgentStatus>('get_status');

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export const SetName = (name: string) => invoke<void>('set_name', { name });
export const GetInstallPath = () => invoke<string>('get_install_path');
export const SetInstallPath = (path: string) => invoke<void>('set_install_path', { path });
export const SelectInstallPath = () => invoke<string>('select_install_path');

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

export const SetAcceptConnections = (accept: boolean) =>
	invoke<void>('set_accept_connections', { accept });
export const DisconnectHub = () => invoke<void>('disconnect_hub');

// ---------------------------------------------------------------------------
// Steam
// ---------------------------------------------------------------------------

export const GetSteamUsers = () => invoke<SteamUserInfo[]>('get_steam_users');
export const GetShortcuts = (userId: string) => invoke<ShortcutInfo[]>('get_shortcuts', { userId });
export const DeleteShortcut = (userId: string, appId: number) =>
	invoke<void>('delete_shortcut', { userId, appId });

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

export const SetTelemetryEnabled = (enabled: boolean) =>
	invoke<void>('set_telemetry_enabled', { enabled });
export const SetTelemetryInterval = (seconds: number) =>
	invoke<void>('set_telemetry_interval', { seconds });

// ---------------------------------------------------------------------------
// Console log
// ---------------------------------------------------------------------------

export const SetConsoleLogEnabled = (enabled: boolean) =>
	invoke<void>('set_console_log_enabled', { enabled });

// ---------------------------------------------------------------------------
// Authorized Hubs
// ---------------------------------------------------------------------------

export const GetAuthorizedHubs = () => invoke<AuthorizedHubDto[]>('get_authorized_hubs');
export const RevokeHub = (hubId: string) => invoke<void>('revoke_hub', { hubId });
