import * as App from '$wailsjs/go/main/App';
import * as runtime from '$wailsjs/runtime/runtime';

// App bindings
export const GetStatus = App.GetStatus;
export const GetSteamUsers = App.GetSteamUsers;
export const GetShortcuts = App.GetShortcuts;
export const SetAcceptConnections = App.SetAcceptConnections;
export const DisconnectHub = App.DisconnectHub;
export const SetName = App.SetName;

// Runtime
export const EventsOn = runtime.EventsOn;
export const EventsOff = runtime.EventsOff;
