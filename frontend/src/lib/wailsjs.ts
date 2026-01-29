// Wails bindings wrapper
// These functions call the Go backend through Wails

declare global {
	interface Window {
		go: {
			main: {
				App: {
					GetDevices(): Promise<any[]>;
					AddDevice(dev: any): Promise<void>;
					UpdateDevice(oldHost: string, dev: any): Promise<void>;
					RemoveDevice(host: string): Promise<void>;
					ConnectDevice(host: string): Promise<void>;
					DisconnectDevice(): Promise<void>;
					GetConnectionStatus(): Promise<any>;
					ScanNetwork(): Promise<any[]>;
					GetGameSetups(): Promise<any[]>;
					AddGameSetup(setup: any): Promise<void>;
					UpdateGameSetup(id: string, setup: any): Promise<void>;
					RemoveGameSetup(id: string): Promise<void>;
					SelectFolder(): Promise<string>;
					UploadGame(setupID: string): Promise<void>;
					GetInstalledGames(remotePath: string): Promise<any[]>;
					DeleteGame(name: string, path: string): Promise<void>;
					GetSteamGridDBAPIKey(): Promise<string>;
					SetSteamGridDBAPIKey(key: string): Promise<void>;
					GetCacheSize(): Promise<number>;
					ClearImageCache(): Promise<void>;
					OpenCacheFolder(): Promise<void>;
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

// Device functions
export const GetDevices = () => window.go.main.App.GetDevices();
export const AddDevice = (dev: any) => window.go.main.App.AddDevice(dev);
export const UpdateDevice = (oldHost: string, dev: any) => window.go.main.App.UpdateDevice(oldHost, dev);
export const RemoveDevice = (host: string) => window.go.main.App.RemoveDevice(host);
export const ConnectDevice = (host: string) => window.go.main.App.ConnectDevice(host);
export const DisconnectDevice = () => window.go.main.App.DisconnectDevice();
export const GetConnectionStatus = () => window.go.main.App.GetConnectionStatus();
export const ScanNetwork = () => window.go.main.App.ScanNetwork();

// Game setup functions
export const GetGameSetups = () => window.go.main.App.GetGameSetups();
export const AddGameSetup = (setup: any) => window.go.main.App.AddGameSetup(setup);
export const UpdateGameSetup = (id: string, setup: any) => window.go.main.App.UpdateGameSetup(id, setup);
export const RemoveGameSetup = (id: string) => window.go.main.App.RemoveGameSetup(id);
export const SelectFolder = () => window.go.main.App.SelectFolder();
export const UploadGame = (setupID: string) => window.go.main.App.UploadGame(setupID);

// Installed games functions
export const GetInstalledGames = (remotePath: string) => window.go.main.App.GetInstalledGames(remotePath);
export const DeleteGame = (name: string, path: string) => window.go.main.App.DeleteGame(name, path);

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
