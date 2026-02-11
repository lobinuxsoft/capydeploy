// Wails bindings wrapper
// Re-exports from generated Wails bindings with type definitions

import * as App from '$wailsjs/go/main/App';
import * as runtime from '$wailsjs/runtime/runtime';

// Agent discovery functions (new)
export const GetDiscoveredAgents = App.GetDiscoveredAgents;
export const RefreshDiscovery = App.RefreshDiscovery;
export const ConnectAgent = App.ConnectAgent;
export const DisconnectAgent = App.DisconnectAgent;
export const GetConnectionStatus = App.GetConnectionStatus;
export const GetAgentInstallPath = App.GetAgentInstallPath;

// Console log
export const SetConsoleLogFilter = App.SetConsoleLogFilter;
export const SetConsoleLogEnabled = App.SetConsoleLogEnabled;

// Pairing functions
export const ConfirmPairing = App.ConfirmPairing;
export const CancelPairing = App.CancelPairing;

// Game setup functions
export const GetGameSetups = App.GetGameSetups;
export const AddGameSetup = App.AddGameSetup;
export const UpdateGameSetup = App.UpdateGameSetup;
export const RemoveGameSetup = App.RemoveGameSetup;
export const SelectFolder = App.SelectFolder;
export const UploadGame = App.UploadGame;

// Installed games functions
export const GetInstalledGames = App.GetInstalledGames;
export const DeleteGame = App.DeleteGame;
export const UpdateGameArtwork = App.UpdateGameArtwork;

// Version
export const GetVersion = App.GetVersion;

// Hub identity functions
export const GetHubInfo = App.GetHubInfo;
export const GetHubName = App.GetHubName;
export const SetHubName = App.SetHubName;

// Settings functions
export const GetSteamGridDBAPIKey = App.GetSteamGridDBAPIKey;
export const SetSteamGridDBAPIKey = App.SetSteamGridDBAPIKey;
export const GetCacheSize = App.GetCacheSize;
export const ClearImageCache = App.ClearImageCache;
export const OpenCacheFolder = App.OpenCacheFolder;
export const GetImageCacheEnabled = App.GetImageCacheEnabled;
export const SetImageCacheEnabled = App.SetImageCacheEnabled;

// Artwork local file functions
export const SelectArtworkFile = App.SelectArtworkFile;
export const GetArtworkPreview = App.GetArtworkPreview;

// SteamGridDB functions
export const SearchGames = App.SearchGames;
export const GetGrids = App.GetGrids;
export const GetHeroes = App.GetHeroes;
export const GetLogos = App.GetLogos;
export const GetIcons = App.GetIcons;

// Runtime events
export const EventsOn = runtime.EventsOn;
export const EventsOff = runtime.EventsOff;
