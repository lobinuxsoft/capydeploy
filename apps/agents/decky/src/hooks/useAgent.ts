/**
 * Hook for managing the CapyDeploy Agent status and configuration.
 * Event polling is handled by background poller in index.tsx.
 */

import { useState, useCallback, useEffect } from "react";
import { call } from "@decky/api";

export interface AgentStatus {
  enabled: boolean;
  connected: boolean;
  hubName: string | null;
  agentName: string;
  installPath: string;
  platform: string;
  version: string;
  port: number;
  ip: string;
}

interface ArtworkAsset {
  data: string;
  format: string;
}

export interface ShortcutConfig {
  name: string;
  exe: string;
  startDir: string;
  iconUrl?: string;
  artwork?: {
    grid?: ArtworkAsset;
    hero?: ArtworkAsset;
    logo?: ArtworkAsset;
    banner?: ArtworkAsset;
  };
}

export interface UseAgentReturn {
  enabled: boolean;
  setEnabled: (enabled: boolean) => Promise<void>;
  status: AgentStatus | null;
  refreshStatus: () => Promise<void>;
  pairingCode: string | null;
  setPairingCode: (code: string | null) => void;
}

export function useAgent(): UseAgentReturn {
  const [enabled, setEnabledState] = useState(false);
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [pairingCode, setPairingCode] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const result = await call<[], AgentStatus>("get_status");
      setStatus(result);
      setEnabledState(result.enabled);
    } catch (e) {
      console.error("Failed to get status:", e);
    }
  }, []);

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

  return {
    enabled,
    setEnabled,
    status,
    refreshStatus,
    pairingCode,
    setPairingCode,
  };
}

export default useAgent;
