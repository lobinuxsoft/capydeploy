/**
 * Hook for managing the CapyDeploy Agent backend.
 * Communicates with main.py via Decky's call() API.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { call } from "@decky/api";
import type { OperationEvent, UploadProgress } from "../types";

export interface AgentStatus {
  enabled: boolean;
  connected: boolean;
  hubName: string | null;
  agentName: string;
  installPath: string;
}

export interface UseAgentOptions {
  onOperation?: (event: OperationEvent) => void;
  onProgress?: (progress: UploadProgress) => void;
  onPairingCode?: (code: string) => void;
  onShortcutRequest?: (config: ShortcutConfig) => void;
}

export interface ShortcutConfig {
  name: string;
  exe: string;
  startDir: string;
  artwork?: {
    grid?: string;
    hero?: string;
    logo?: string;
    icon?: string;
  };
}

export interface UseAgentReturn {
  enabled: boolean;
  setEnabled: (enabled: boolean) => Promise<void>;
  status: AgentStatus | null;
  refreshStatus: () => Promise<void>;
  pairingCode: string | null;
}

// Polling interval for events (ms)
const POLL_INTERVAL = 1000;

export function useAgent(options: UseAgentOptions = {}): UseAgentReturn {
  const { onOperation, onProgress, onPairingCode, onShortcutRequest } = options;

  const [enabled, setEnabledState] = useState(false);
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Fetch current status from backend
  const refreshStatus = useCallback(async () => {
    try {
      const result = await call<[], AgentStatus>("get_status");
      setStatus(result);
      setEnabledState(result.enabled);
    } catch (e) {
      console.error("Failed to get status:", e);
    }
  }, []);

  // Poll for events from backend
  const pollEvents = useCallback(async () => {
    try {
      // Check for operation events
      const opEvent = await call<[string], { timestamp: number; data: OperationEvent } | null>(
        "get_event",
        "operation_event"
      );
      if (opEvent?.data) {
        onOperation?.(opEvent.data);
      }

      // Check for upload progress
      const progressEvent = await call<[string], { timestamp: number; data: UploadProgress } | null>(
        "get_event",
        "upload_progress"
      );
      if (progressEvent?.data) {
        onProgress?.(progressEvent.data);
      }

      // Check for pairing code
      const pairingEvent = await call<[string], { timestamp: number; data: { code: string } } | null>(
        "get_event",
        "pairing_code"
      );
      if (pairingEvent?.data) {
        setPairingCode(pairingEvent.data.code);
        onPairingCode?.(pairingEvent.data.code);
      }

      // Check for pairing success
      const pairingSuccess = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "pairing_success"
      );
      if (pairingSuccess) {
        setPairingCode(null);
        refreshStatus();
      }

      // Check for hub connected/disconnected
      const hubConnected = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "hub_connected"
      );
      if (hubConnected) {
        refreshStatus();
      }

      const hubDisconnected = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "hub_disconnected"
      );
      if (hubDisconnected) {
        refreshStatus();
      }

      // Check for shortcut creation request
      const shortcutEvent = await call<[string], { timestamp: number; data: ShortcutConfig } | null>(
        "get_event",
        "create_shortcut"
      );
      if (shortcutEvent?.data) {
        onShortcutRequest?.(shortcutEvent.data);
      }
    } catch (e) {
      console.error("Failed to poll events:", e);
    }
  }, [onOperation, onProgress, onPairingCode, onShortcutRequest, refreshStatus]);

  // Enable/disable the server
  const setEnabled = useCallback(async (value: boolean) => {
    try {
      await call<[boolean], void>("set_enabled", value);
      setEnabledState(value);
      await refreshStatus();
    } catch (e) {
      console.error("Failed to set enabled:", e);
    }
  }, [refreshStatus]);

  // Initial load
  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  // Start/stop polling based on enabled state
  useEffect(() => {
    if (enabled) {
      pollRef.current = setInterval(pollEvents, POLL_INTERVAL);
    } else {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    }

    return () => {
      if (pollRef.current) {
        clearInterval(pollRef.current);
      }
    };
  }, [enabled, pollEvents]);

  return {
    enabled,
    setEnabled,
    status,
    refreshStatus,
    pairingCode,
  };
}

export default useAgent;
