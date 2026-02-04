import * as App from '$wailsjs/go/main/App';
import * as runtime from '$wailsjs/runtime/runtime';

// App bindings
export const GetVersion = App.GetVersion;
export const GetStatus = App.GetStatus;
export const GetSteamUsers = App.GetSteamUsers;
export const GetShortcuts = App.GetShortcuts;
export const DeleteShortcut = App.DeleteShortcut;
export const SetAcceptConnections = App.SetAcceptConnections;
export const DisconnectHub = App.DisconnectHub;
export const SetName = App.SetName;
export const GetInstallPath = App.GetInstallPath;
export const SetInstallPath = App.SetInstallPath;
export const SelectInstallPath = App.SelectInstallPath;
export const GetAuthorizedHubs = App.GetAuthorizedHubs;
export const RevokeHub = App.RevokeHub;

// Runtime
export const EventsOn = runtime.EventsOn;
export const EventsOff = runtime.EventsOff;
