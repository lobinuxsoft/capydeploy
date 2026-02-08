/**
 * Background event polling and SteamClient operation handlers.
 * Runs even when the QAM panel is closed.
 */

import { showModal } from "@decky/ui";
import { call, toaster } from "@decky/api";

import type { ShortcutConfig } from "./hooks/useAgent";
import { ProgressModalContent, progressState } from "./components/ProgressPanel";
import ConfirmActionModal from "./components/ConfirmActionModal";
import PairingCodeModal from "./components/PairingCodeModal";
import type { OperationEvent, UploadProgress } from "./types";

// Import mascot for branded toasts
import mascotUrl from "../assets/mascot.gif";

// Asset types for SetCustomArtworkForApp (values from decky-steamgriddb)
const ASSET_TYPE = {
  grid_p: 0, // Portrait grid / Capsule (600x900)
  hero: 1,   // Hero (1920x620)
  logo: 2,   // Logo
  grid_l: 3, // Landscape grid / Wide Capsule (920x430)
  icon: 4,   // Icon
};

// ── UI callback registry (React registers when panel is open) ──────────────

export interface UICallbacks {
  onOperation?: (event: OperationEvent) => void;
  onProgress?: (progress: UploadProgress) => void;
  onPairingCode?: (code: string) => void;
  onPairingClear?: () => void;
  onRefreshStatus?: () => void;
}

let _uiCallbacks: UICallbacks = {};

export function registerUICallbacks(cbs: UICallbacks) {
  _uiCallbacks = cbs;
}

export function unregisterUICallbacks() {
  _uiCallbacks = {};
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

      // Set Proton for Windows executables (Decky always runs on Linux)
      if (config.exe.toLowerCase().endsWith(".exe")) {
        try {
          SteamClient.Apps.SpecifyCompatTool(appId, "proton_experimental");
        } catch (e) {
          console.warn("Failed to set Proton:", e);
        }
      }

      await call<[string, number], void>("register_shortcut", config.name, appId);

      // Apply artwork (backend sends {data: base64, format: "png"|"jpg"})
      if (config.artwork) {
        const artworkEntries: [{ data: string; format: string } | undefined, number][] = [
          [config.artwork.grid, ASSET_TYPE.grid_p],
          [config.artwork.hero, ASSET_TYPE.hero],
          [config.artwork.logo, ASSET_TYPE.logo],
          [config.artwork.banner, ASSET_TYPE.grid_l],
        ];
        for (const [art, assetType] of artworkEntries) {
          if (art?.data) {
            try {
              await SteamClient.Apps.ClearCustomArtworkForApp(appId, assetType);
              await new Promise(r => setTimeout(r, 500));
              await SteamClient.Apps.SetCustomArtworkForApp(
                appId,
                art.data,
                "png",
                assetType
              );

              // Force default logo position to prevent invisible logos
              if (assetType === ASSET_TYPE.logo) {
                const appOverview = window.appStore?.GetAppOverviewByAppID(appId);
                if (appOverview) {
                  await window.appDetailsStore?.SaveCustomLogoPosition(appOverview as any, {
                    pinnedPosition: "BottomLeft",
                    nWidthPct: 50,
                    nHeightPct: 50,
                  });
                }
              }
            } catch (e) {
              console.error(`Failed to apply artwork (type ${assetType}):`, e);
            }
          }
        }

      }

      // Icons: backend downloads directly from URL (no base64 round-trip)
      if (config.iconUrl) {
        try {
          await call<[number, string], boolean>(
            "set_shortcut_icon_from_url",
            appId,
            config.iconUrl
          );
        } catch (e) {
          console.error("Failed to set shortcut icon:", e);
        }
      }

    }
  } catch (e) {
    console.error("Failed to create shortcut:", e);
    brandToast({ title: "Shortcut error", body: String(e) });
  }
}

function handleRemoveShortcut(appId: number) {
  try {
    SteamClient.Apps.RemoveShortcut(appId);
  } catch (e) {
    console.error("Failed to remove shortcut:", e);
  }
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
    // ── SteamClient operations (critical, drain full queue) ──

    let shortcutEvent;
    do {
      shortcutEvent = await call<[string], { timestamp: number; data: ShortcutConfig } | null>(
        "get_event",
        "create_shortcut"
      );
      if (shortcutEvent?.data) {
        handleCreateShortcut(shortcutEvent.data);
      }
    } while (shortcutEvent?.data);

    let removeEvent;
    do {
      removeEvent = await call<[string], { timestamp: number; data: { appId: number } } | null>(
        "get_event",
        "remove_shortcut"
      );
      if (removeEvent?.data) {
        handleRemoveShortcut(removeEvent.data.appId);
      }
    } while (removeEvent?.data);

    // ── Operation events (drain queue: toasts always, UI state when panel open) ──

    let opEvent;
    do {
      opEvent = await call<[string], { timestamp: number; data: OperationEvent } | null>(
        "get_event",
        "operation_event"
      );
      if (opEvent?.data) {
        const event = opEvent.data;
        _uiCallbacks.onOperation?.(event);

        if (event.status === "start") {
          progressState.update(event, null);
          showProgressModal();
        } else if (event.status === "complete") {
          progressState.update(event, null);
          closeProgressModal();
          brandToast({
            title: event.type === "install" ? "Game installed!" : "Game removed",
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
    } while (opEvent?.data);

    // ── Upload progress (overwrite-based, only latest matters) ──

    const progressEvent = await call<[string], { timestamp: number; data: UploadProgress } | null>(
      "get_event",
      "upload_progress"
    );
    if (progressEvent?.data) {
      _uiCallbacks.onProgress?.(progressEvent.data);
      progressState.update(progressState.operation, progressEvent.data);
    }

    // ── Pairing code (drain queue) ──

    let pairingEvent;
    do {
      pairingEvent = await call<[string], { timestamp: number; data: { code: string } } | null>(
        "get_event",
        "pairing_code"
      );
      if (pairingEvent?.data) {
        const code = pairingEvent.data.code;
        _uiCallbacks.onPairingCode?.(code);
        showModal(<PairingCodeModal code={code} />);
      }
    } while (pairingEvent?.data);

    // ── Pairing success (drain queue) ──

    let pairingSuccess;
    do {
      pairingSuccess = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "pairing_success"
      );
      if (pairingSuccess?.data) {
        _uiCallbacks.onPairingClear?.();
        _uiCallbacks.onRefreshStatus?.();
        brandToast({
          title: "Hub linked!",
          body: "Pairing successful",
        });
      }
    } while (pairingSuccess?.data);

    // ── Hub connection state changes (drain queue) ──

    let hubConnected;
    do {
      hubConnected = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "hub_connected"
      );
      if (hubConnected?.data) {
        _uiCallbacks.onRefreshStatus?.();
      }
    } while (hubConnected?.data);

    let hubDisconnected;
    do {
      hubDisconnected = await call<[string], { timestamp: number; data: object } | null>(
        "get_event",
        "hub_disconnected"
      );
      if (hubDisconnected?.data) {
        _uiCallbacks.onRefreshStatus?.();
      }
    } while (hubDisconnected?.data);

    // ── Server error (queue-based, show modal) ──

    let serverError;
    do {
      serverError = await call<[string], { timestamp: number; data: { message: string } } | null>(
        "get_event",
        "server_error"
      );
      if (serverError?.data) {
        showModal(
          <ConfirmActionModal
            title="Server Error"
            description={`${serverError.data.message}\n\nTry reinstalling the plugin or check the logs.`}
            confirmText="OK"
            onConfirm={() => {}}
          />
        );
      }
    } while (serverError?.data);
  } catch (e) {
    console.error("Background poll error:", e);
  }
}

export function startBackgroundPolling() {
  if (!bgPollInterval) {
    bgPollInterval = setInterval(pollAllEvents, 1000);
  }
}

export function stopBackgroundPolling() {
  if (bgPollInterval) {
    clearInterval(bgPollInterval);
    bgPollInterval = null;
  }
}
