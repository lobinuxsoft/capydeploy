/**
 * CapyDeploy Decky Plugin
 * Receive games from your PC and create Steam shortcuts in gaming mode.
 */

import { definePlugin, staticClasses } from "@decky/ui";
import { call, toaster } from "@decky/api";
import { useState, useCallback, VFC } from "react";

import { useAgent, ShortcutConfig } from "./hooks/useAgent";
import StatusPanel from "./components/StatusPanel";
import AuthorizedHubs from "./components/AuthorizedHubs";
import InstalledGames from "./components/InstalledGames";
import ProgressPanel from "./components/ProgressPanel";
import CapyIcon from "./components/CapyIcon";
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

const CapyDeployPanel: VFC = () => {
  const [currentOperation, setCurrentOperation] = useState<OperationEvent | null>(null);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);
  const [gamesRefresh, setGamesRefresh] = useState(0);

  // Handle shortcut creation using SteamClient API
  const handleShortcutRequest = useCallback(async (config: ShortcutConfig) => {
    try {
      // Create the shortcut
      const appId = await SteamClient.Apps.AddShortcut(
        config.name,
        config.exe,
        config.startDir,
        ""
      );

      if (appId) {
        // Set the correct name (sometimes AddShortcut doesn't set it right)
        SteamClient.Apps.SetShortcutName(appId, config.name);

        // Register appId in backend for Hub queries
        await call<[string, number], void>("register_shortcut", config.name, appId);

        // Apply artwork if provided (backend sends {data: base64, format: "png"|"jpg"})
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

        toaster.toast({
          title: "Shortcut creado!",
          body: config.name,
        });
      }
    } catch (e) {
      console.error("Failed to create shortcut:", e);
      toaster.toast({
        title: "Error al crear shortcut",
        body: String(e),
      });
    }
  }, []);

  const { enabled, setEnabled, status, pairingCode, refreshStatus } = useAgent({
    onOperation: (event) => {
      setCurrentOperation(event);

      // Show toast notifications
      if (event.status === "start") {
        toaster.toast({
          title: event.type === "install" ? "Instalando juego" : "Eliminando juego",
          body: event.gameName,
        });
      } else if (event.status === "complete") {
        toaster.toast({
          title: event.type === "install" ? "Juego instalado!" : "Juego eliminado",
          body: event.gameName,
        });
        setGamesRefresh((n) => n + 1);
        // Clear operation after a delay
        setTimeout(() => setCurrentOperation(null), 5000);
      } else if (event.status === "error") {
        toaster.toast({
          title: "Error",
          body: `${event.gameName}: ${event.message}`,
        });
      }
    },
    onProgress: (progress) => {
      setUploadProgress(progress);
    },
    onPairingCode: (code) => {
      toaster.toast({
        title: "Codigo de emparejamiento",
        body: code,
      });
    },
    onShortcutRequest: handleShortcutRequest,
    onRemoveShortcut: (appId) => {
      try {
        SteamClient.Apps.RemoveShortcut(appId);
      } catch (e) {
        console.error("Failed to remove shortcut:", e);
      }
    },
  });

  return (
    <div>
      {/* Header with mascot */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "12px",
          padding: "12px",
          marginBottom: "8px",
        }}
      >
        <img
          src={mascotUrl}
          alt="CapyDeploy"
          style={{
            width: "64px",
            height: "64px",
            borderRadius: "12px",
            objectFit: "cover",
            border: "2px solid rgba(89, 191, 64, 0.5)",
          }}
        />
        <div>
          <div style={{ fontWeight: "bold", fontSize: "1.1em" }}>CapyDeploy</div>
          <div style={{ fontSize: "0.8em", opacity: 0.7 }}>
            Recibe juegos desde el Hub
          </div>
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
  return {
    title: <div className={staticClasses.Title}>CapyDeploy</div>,
    content: <CapyDeployPanel />,
    icon: <CapyIcon />,
    onDismount() {
      // Cleanup if needed
    },
  };
});
