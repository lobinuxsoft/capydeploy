/**
 * CapyDeploy Decky Plugin
 * Receive games from your PC and create Steam shortcuts in gaming mode.
 */

import { definePlugin, staticClasses, showModal, ConfirmModal } from "@decky/ui";
import { call, toaster } from "@decky/api";
import { useState, useEffect, VFC } from "react";

import { useAgent, ShortcutConfig } from "./hooks/useAgent";
import StatusPanel from "./components/StatusPanel";
import AuthorizedHubs from "./components/AuthorizedHubs";
import InstalledGames from "./components/InstalledGames";
import ProgressPanel, { ProgressModalContent, progressState } from "./components/ProgressPanel";
import CapyIcon from "./components/CapyIcon";
import { getThemeCSS } from "./styles/theme";
import type { OperationEvent, UploadProgress } from "./types";

// Import mascot
import mascotUrl from "../assets/mascot.gif";

// Declare SteamClient global (injected by Steam)
declare const SteamClient: {
  Apps: {
    AddShortcut: (name: string, exe: string, startDir: string, launchOptions: string) => Promise<number>;
    SetShortcutName: (appId: number, name: string) => void;
    SetCustomArtworkForApp: (appId: number, data: string, format: string, assetType: number) => Promise<void>;
    RemoveShortcut: (appId: number) => void;
  };
};

// Asset types for SetCustomArtworkForApp (values from decky-steamgriddb)
const ASSET_TYPE = {
  grid_p: 0, // Portrait grid / Capsule (600x900)
  hero: 1,   // Hero (1920x620)
  logo: 2,   // Logo
  grid_l: 3, // Landscape grid / Wide Capsule (920x430)
  icon: 4,   // Icon
};

// ── UI callback registry (React registers when panel is open) ──────────────

interface UICallbacks {
  onOperation?: (event: OperationEvent) => void;
  onProgress?: (progress: UploadProgress) => void;
  onPairingCode?: (code: string) => void;
  onPairingClear?: () => void;
  onRefreshStatus?: () => void;
}

let _uiCallbacks: UICallbacks = {};

function registerUICallbacks(cbs: UICallbacks) {
  _uiCallbacks = cbs;
}

function unregisterUICallbacks() {
  _uiCallbacks = {};
}

// ── Background handlers for SteamClient operations ─────────────────────────

async function handleCreateShortcut(config: ShortcutConfig) {
  try {
    const appId = await SteamClient.Apps.AddShortcut(
      config.name,
      config.exe,
      config.startDir,
      ""
    );

    if (appId) {
      SteamClient.Apps.SetShortcutName(appId, config.name);
      await call<[string, number], void>("register_shortcut", config.name, appId);

      // Apply artwork (backend sends {data: base64, format: "png"|"jpg"})
      if (config.artwork) {
        const artworkEntries: [{ data: string; format: string } | undefined, number][] = [
          [config.artwork.grid, ASSET_TYPE.grid_p],
          [config.artwork.hero, ASSET_TYPE.hero],
          [config.artwork.logo, ASSET_TYPE.logo],
        ];
        for (const [art, assetType] of artworkEntries) {
          if (art?.data) {
            try {
              await SteamClient.Apps.SetCustomArtworkForApp(
                appId,
                art.data,
                art.format || "png",
                assetType
              );
            } catch (e) {
              console.error(`Failed to apply artwork (type ${assetType}):`, e);
            }
          }
        }
      }

      brandToast({ title: "Shortcut creado!", body: config.name });
    }
  } catch (e) {
    console.error("Failed to create shortcut:", e);
    brandToast({ title: "Error al crear shortcut", body: String(e) });
  }
}

function handleRemoveShortcut(appId: number) {
  try {
    SteamClient.Apps.RemoveShortcut(appId);
  } catch (e) {
    console.error("Failed to remove shortcut:", e);
  }
}

// ── Branded toast helper ────────────────────────────────────────────────────

const toastLogo = (
  <img
    src={mascotUrl}
    style={{ width: "100%", height: "100%", borderRadius: "50%", objectFit: "cover" }}
  />
);

function brandToast(opts: { title: string; body: string }) {
  toaster.toast({ ...opts, logo: toastLogo });
}

// ── Progress modal management ──────────────────────────────────────────────

let progressModalHandle: { Close: () => void } | null = null;

function showProgressModal() {
  if (!progressModalHandle) {
    progressModalHandle = showModal(<ProgressModalContent />);
  }
}

function closeProgressModal(delay = 3000) {
  setTimeout(() => {
    progressModalHandle?.Close();
    progressModalHandle = null;
  }, delay);
}

// ── Centralized background polling (runs even when panel is closed) ────────

let bgPollInterval: ReturnType<typeof setInterval> | null = null;

async function pollAllEvents() {
  try {
    // ── SteamClient operations (critical, must run in background) ──

    const shortcutEvent = await call<[string], { timestamp: number; data: ShortcutConfig } | null>(
      "get_event",
      "create_shortcut"
    );
    if (shortcutEvent?.data) {
      handleCreateShortcut(shortcutEvent.data);
    }

    const removeEvent = await call<[string], { timestamp: number; data: { appId: number } } | null>(
      "get_event",
      "remove_shortcut"
    );
    if (removeEvent?.data) {
      handleRemoveShortcut(removeEvent.data.appId);
    }

    // ── Operation events (toasts always, UI state when panel open) ──

    const opEvent = await call<[string], { timestamp: number; data: OperationEvent } | null>(
      "get_event",
      "operation_event"
    );
    if (opEvent?.data) {
      const event = opEvent.data;
      _uiCallbacks.onOperation?.(event);

      if (event.status === "start") {
        progressState.update(event, null);
        showProgressModal();
        brandToast({
          title: event.type === "install" ? "Instalando juego" : "Eliminando juego",
          body: event.gameName,
        });
      } else if (event.status === "complete") {
        progressState.update(event, null);
        closeProgressModal();
        brandToast({
          title: event.type === "install" ? "Juego instalado!" : "Juego eliminado",
          body: event.gameName,
        });
      } else if (event.status === "error") {
        progressState.update(event, null);
        closeProgressModal(5000);
        brandToast({
          title: "Error",
          body: `${event.gameName}: ${event.message}`,
        });
      } else {
        progressState.update(event, progressState.progress);
      }
    }

    // ── Upload progress (UI state + modal update) ──

    const progressEvent = await call<[string], { timestamp: number; data: UploadProgress } | null>(
      "get_event",
      "upload_progress"
    );
    if (progressEvent?.data) {
      _uiCallbacks.onProgress?.(progressEvent.data);
      progressState.update(progressState.operation, progressEvent.data);
    }

    // ── Pairing code (persistent modal, UI state when panel open) ──

    const pairingEvent = await call<[string], { timestamp: number; data: { code: string } } | null>(
      "get_event",
      "pairing_code"
    );
    if (pairingEvent?.data) {
      const code = pairingEvent.data.code;
      _uiCallbacks.onPairingCode?.(code);
      showModal(
        <ConfirmModal
          strTitle="Codigo de emparejamiento"
          strDescription={`Ingresa este codigo en el Hub para vincular este dispositivo:\n\n${code}`}
          strOKButtonText="Entendido"
          strCancelButtonText="Cerrar"
        />
      );
    }

    // ── Pairing success (clear code, refresh status) ──

    const pairingSuccess = await call<[string], { timestamp: number; data: object } | null>(
      "get_event",
      "pairing_success"
    );
    if (pairingSuccess) {
      _uiCallbacks.onPairingClear?.();
      _uiCallbacks.onRefreshStatus?.();
      brandToast({
        title: "Hub vinculado!",
        body: "Emparejamiento exitoso",
      });
    }

    // ── Hub connection state changes ──

    const hubConnected = await call<[string], { timestamp: number; data: object } | null>(
      "get_event",
      "hub_connected"
    );
    if (hubConnected) {
      _uiCallbacks.onRefreshStatus?.();
    }

    const hubDisconnected = await call<[string], { timestamp: number; data: object } | null>(
      "get_event",
      "hub_disconnected"
    );
    if (hubDisconnected) {
      _uiCallbacks.onRefreshStatus?.();
    }
  } catch (e) {
    console.error("Background poll error:", e);
  }
}

function startBackgroundPolling() {
  if (!bgPollInterval) {
    bgPollInterval = setInterval(pollAllEvents, 1000);
  }
}

function stopBackgroundPolling() {
  if (bgPollInterval) {
    clearInterval(bgPollInterval);
    bgPollInterval = null;
  }
}

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
          <div className="cd-subtitle">Recibe juegos desde el Hub</div>
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
