/**
 * CapyDeploy Decky Plugin
 * Receive games from your PC and create Steam shortcuts in gaming mode.
 */

import { definePlugin, staticClasses } from "@decky/ui";
import { useState, useEffect, VFC } from "react";

import { useAgent } from "./hooks/useAgent";
import StatusPanel from "./components/StatusPanel";
import AuthorizedHubs from "./components/AuthorizedHubs";
import InstalledGames from "./components/InstalledGames";
import ProgressPanel from "./components/ProgressPanel";
import CapyIcon from "./components/CapyIcon";
import { getThemeCSS } from "./styles/theme";
import {
  registerUICallbacks,
  unregisterUICallbacks,
  startBackgroundPolling,
  stopBackgroundPolling,
} from "./eventPoller";
import type { OperationEvent, UploadProgress } from "./types";

// Import mascot
import mascotUrl from "../assets/mascot.gif";

// ── React UI component ─────────────────────────────────────────────────────

const CapyDeployPanel: VFC = () => {
  const [currentOperation, setCurrentOperation] = useState<OperationEvent | null>(null);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);
  const [gamesRefresh, setGamesRefresh] = useState(0);

  const { enabled, setEnabled, status, pairingCode, setPairingCode, refreshStatus } = useAgent();

  // Register UI callbacks so background poller can update React state
  useEffect(() => {
    registerUICallbacks({
      onOperation: (event) => {
        setCurrentOperation(event);
        if (event.status === "complete") {
          setGamesRefresh((n) => n + 1);
          setTimeout(() => setCurrentOperation(null), 5000);
        }
      },
      onProgress: (progress) => setUploadProgress(progress),
      onPairingCode: (code) => setPairingCode(code),
      onPairingClear: () => setPairingCode(null),
      onRefreshStatus: () => refreshStatus(),
    });

    return () => unregisterUICallbacks();
  }, [setPairingCode, refreshStatus]);

  return (
    <div id="capydeploy-wrap">
      <style>{getThemeCSS()}</style>

      {/* Header with mascot */}
      <div className="cd-header">
        <div className="cd-mascot-wrap">
          <img src={mascotUrl} alt="CapyDeploy" />
        </div>
        <div>
          <div className="cd-title">CapyDeploy</div>
          <div className="cd-subtitle">Receive games from the Hub</div>
        </div>
      </div>

      <StatusPanel
        enabled={enabled}
        onEnabledChange={setEnabled}
        connected={status?.connected ?? false}
        hubName={status?.hubName ?? null}
        pairingCode={pairingCode}
        agentName={status?.agentName ?? "CapyDeploy Agent"}
        platform={status?.platform ?? "linux"}
        version={status?.version ?? "0.1.0"}
        port={status?.port ?? 9999}
        ip={status?.ip ?? "127.0.0.1"}
        installPath={status?.installPath ?? "~/Games"}
        onRefresh={refreshStatus}
      />

      <AuthorizedHubs enabled={enabled} />

      <InstalledGames enabled={enabled} installPath={status?.installPath ?? ""} refreshTrigger={gamesRefresh} />

      <ProgressPanel operation={currentOperation} uploadProgress={uploadProgress} />
    </div>
  );
};

export default definePlugin(() => {
  startBackgroundPolling();

  return {
    title: <div className={staticClasses.Title}>CapyDeploy</div>,
    content: <CapyDeployPanel />,
    icon: <CapyIcon />,
    onDismount() {
      stopBackgroundPolling();
    },
  };
});
