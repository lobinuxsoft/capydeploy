/**
 * WebSocket message types matching the Agent protocol.
 */

export type MessageType =
  | "hub_connected"
  | "agent_status"
  | "pairing_required"
  | "pair_confirm"
  | "pair_success"
  | "pair_failed"
  | "ping"
  | "pong"
  | "get_info"
  | "info_response"
  | "get_steam_users"
  | "steam_users_response"
  | "list_shortcuts"
  | "shortcuts_response"
  | "operation_event"
  | "upload_progress"
  | "error";

export interface WSMessage {
  id: string;
  type: MessageType;
  payload?: unknown;
  error?: {
    code: number;
    message: string;
  };
}

export interface AgentInfo {
  id: string;
  name: string;
  platform: string;
  version: string;
  acceptConnections: boolean;
}

export interface HubConnectedPayload {
  name: string;
  version: string;
  hubId: string;
  token?: string;
}

export interface AgentStatusPayload {
  name: string;
  version: string;
  platform: string;
  acceptConnections: boolean;
}

export interface PairingRequiredPayload {
  code: string;
  expiresIn: number;
}

export interface OperationEvent {
  type: "install" | "delete";
  status: "start" | "progress" | "complete" | "error";
  gameName: string;
  progress: number;
  message?: string;
}

export interface UploadProgress {
  uploadId: string;
  transferredBytes: number;
  totalBytes: number;
  currentFile?: string;
  percentage: number;
}

export interface ShortcutInfo {
  appId: number;
  name: string;
  exe: string;
  startDir: string;
  launchOptions?: string;
  tags?: string[];
  lastPlayed?: number;
}

export interface ConnectionState {
  connected: boolean;
  authorized: boolean;
  agentName?: string;
  agentVersion?: string;
  pairingCode?: string;
}
