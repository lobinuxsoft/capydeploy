/**
 * CapyDeploy Decky Plugin
 * Receive games from your PC and create Steam shortcuts in gaming mode.
 */

import { definePlugin, staticClasses } from "@decky/ui";
import { toaster } from "@decky/api";
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

// Asset types for SetCustomArtworkForApp
const ASSET_TYPE = {
  grid_p: 0, // Portrait grid (600x900)
  grid_l: 1, // Landscape grid (920x430)
  hero: 2,   // Hero (1920x620)
  logo: 3,   // Logo
  icon: 4,   // Icon
};

const CapyDeployPanel: VFC = () => {
  const [currentOperation, setCurrentOperation] = useState<OperationEvent | null>(null);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);

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

        // Apply artwork if provided
        if (config.artwork) {
          if (config.artwork.grid) {
            await SteamClient.Apps.SetCustomArtworkForApp(
              appId,
              config.artwork.grid,
              "png",
              ASSET_TYPE.grid_p
            );
          }
          if (config.artwork.hero) {
            await SteamClient.Apps.SetCustomArtworkForApp(
              appId,
              config.artwork.hero,
              "png",
              ASSET_TYPE.hero
            );
          }
          if (config.artwork.logo) {
            await SteamClient.Apps.SetCustomArtworkForApp(
              appId,
              config.artwork.logo,
              "png",
              ASSET_TYPE.logo
            );
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

      <InstalledGames enabled={enabled} installPath={status?.installPath ?? ""} />

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
