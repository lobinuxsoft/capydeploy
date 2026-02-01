// Wails bindings wrapper
// These functions call the Go backend through Wails

import type { DiscoveredAgent, ConnectionStatus } from './types';

declare global {
	interface Window {
		go: {
			main: {
				App: {
					// Agent discovery (new)
					GetDiscoveredAgents(): Promise<DiscoveredAgent[]>;
					RefreshDiscovery(): Promise<DiscoveredAgent[]>;
					ConnectAgent(agentID: string): Promise<void>;
					DisconnectAgent(): Promise<void>;
					GetConnectionStatus(): Promise<ConnectionStatus>;

					// Legacy device functions (deprecated)
					GetDevices(): Promise<any[]>;
					ConnectDevice(host: string): Promise<void>;
					DisconnectDevice(): Promise<void>;

					// Game setup functions
					GetGameSetups(): Promise<any[]>;
					AddGameSetup(setup: any): Promise<void>;
					UpdateGameSetup(id: string, setup: any): Promise<void>;
					RemoveGameSetup(id: string): Promise<void>;
					SelectFolder(): Promise<string>;
					UploadGame(setupID: string): Promise<void>;

					// Installed games functions
					GetInstalledGames(remotePath: string): Promise<any[]>;
					DeleteGame(name: string, appID: number): Promise<void>;

					// Settings functions
					GetSteamGridDBAPIKey(): Promise<string>;
					SetSteamGridDBAPIKey(key: string): Promise<void>;
					GetCacheSize(): Promise<number>;
					ClearImageCache(): Promise<void>;
					OpenCacheFolder(): Promise<void>;

					// SteamGridDB functions
					SearchGames(query: string): Promise<any[]>;
					GetGrids(gameID: number, filters: any, page: number): Promise<any[]>;
					GetHeroes(gameID: number, filters: any, page: number): Promise<any[]>;
					GetLogos(gameID: number, filters: any, page: number): Promise<any[]>;
					GetIcons(gameID: number, filters: any, page: number): Promise<any[]>;
					ProxyImage(imageURL: string): Promise<string>;
				};
			};
		};
		runtime: {
			EventsOn(event: string, callback: (...args: any[]) => void): void;
			EventsOff(event: string): void;
		};
	}
}

// Agent discovery functions (new)
export const GetDiscoveredAgents = () => window.go.main.App.GetDiscoveredAgents();
export const RefreshDiscovery = () => window.go.main.App.RefreshDiscovery();
export const ConnectAgent = (agentID: string) => window.go.main.App.ConnectAgent(agentID);
export const DisconnectAgent = () => window.go.main.App.DisconnectAgent();
export const GetConnectionStatus = () => window.go.main.App.GetConnectionStatus();

// Legacy device functions (deprecated - use Agent functions instead)
export const GetDevices = () => window.go.main.App.GetDevices();
export const ConnectDevice = (host: string) => window.go.main.App.ConnectDevice(host);
export const DisconnectDevice = () => window.go.main.App.DisconnectAgent();

// Game setup functions
export const GetGameSetups = () => window.go.main.App.GetGameSetups();
export const AddGameSetup = (setup: any) => window.go.main.App.AddGameSetup(setup);
export const UpdateGameSetup = (id: string, setup: any) => window.go.main.App.UpdateGameSetup(id, setup);
export const RemoveGameSetup = (id: string) => window.go.main.App.RemoveGameSetup(id);
export const SelectFolder = () => window.go.main.App.SelectFolder();
export const UploadGame = (setupID: string) => window.go.main.App.UploadGame(setupID);

// Installed games functions
export const GetInstalledGames = (remotePath: string) => window.go.main.App.GetInstalledGames(remotePath);
export const DeleteGame = (name: string, appID: number) => window.go.main.App.DeleteGame(name, appID);

// Settings functions
export const GetSteamGridDBAPIKey = () => window.go.main.App.GetSteamGridDBAPIKey();
export const SetSteamGridDBAPIKey = (key: string) => window.go.main.App.SetSteamGridDBAPIKey(key);
export const GetCacheSize = () => window.go.main.App.GetCacheSize();
export const ClearImageCache = () => window.go.main.App.ClearImageCache();
export const OpenCacheFolder = () => window.go.main.App.OpenCacheFolder();

// SteamGridDB functions
export const SearchGames = (query: string) => window.go.main.App.SearchGames(query);
export const GetGrids = (gameID: number, filters: any, page: number) => window.go.main.App.GetGrids(gameID, filters, page);
export const GetHeroes = (gameID: number, filters: any, page: number) => window.go.main.App.GetHeroes(gameID, filters, page);
export const GetLogos = (gameID: number, filters: any, page: number) => window.go.main.App.GetLogos(gameID, filters, page);
export const GetIcons = (gameID: number, filters: any, page: number) => window.go.main.App.GetIcons(gameID, filters, page);
export const ProxyImage = (imageURL: string) => window.go.main.App.ProxyImage(imageURL);

// Runtime events
export const EventsOn = (event: string, callback: (...args: any[]) => void) => window.runtime.EventsOn(event, callback);
export const EventsOff = (event: string) => window.runtime.EventsOff(event);
