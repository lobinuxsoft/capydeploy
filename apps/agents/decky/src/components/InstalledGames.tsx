/**
 * InstalledGames - Shows games installed by CapyDeploy with uninstall option.
 */

import {
  PanelSection,
  PanelSectionRow,
  Field,
  Focusable,
  showModal,
} from "@decky/ui";
import { call, toaster } from "@decky/api";
import { VFC, useState, useEffect, useCallback } from "react";
import { FaGamepad, FaTrash, FaFolderOpen } from "react-icons/fa6";
import { colors } from "../styles/theme";
import ConfirmActionModal from "./ConfirmActionModal";

import mascotUrl from "../../assets/mascot.gif";

const toastLogo = (
  <img
    src={mascotUrl}
    style={{ width: "100%", height: "100%", borderRadius: "50%", objectFit: "cover" }}
  />
);

interface InstalledGame {
  name: string;
  path: string;
  size: number;
}

interface InstalledGamesProps {
  enabled: boolean;
  installPath: string;
  refreshTrigger?: number;
}

const InstalledGames: VFC<InstalledGamesProps> = ({ enabled, installPath, refreshTrigger }) => {
  const [games, setGames] = useState<InstalledGame[]>([]);
  const [loading, setLoading] = useState(true);
  const [uninstalling, setUninstalling] = useState<string | null>(null);

  const loadGames = useCallback(async () => {
    try {
      const result = await call<[], InstalledGame[]>("get_installed_games");
      setGames(result || []);
    } catch (e) {
      console.error("Failed to load games:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      loadGames();
    }
  }, [enabled, loadGames, installPath, refreshTrigger]);

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const doUninstall = async (game: InstalledGame) => {
    setUninstalling(game.name);
    try {
      const result = await call<[string], number | boolean>("uninstall_game", game.name);
      if (result) {
        if (typeof result === "number" && result > 0) {
          try {
            SteamClient.Apps.RemoveShortcut(result);
          } catch (e) {
            console.error("Failed to remove shortcut:", e);
          }
        }
        setGames(games.filter((g) => g.name !== game.name));
        toaster.toast({ title: "Game removed", body: game.name, logo: toastLogo });
      } else {
        toaster.toast({ title: "Error", body: `Failed to remove ${game.name}`, logo: toastLogo });
      }
    } catch (e) {
      console.error("Failed to uninstall:", e);
      toaster.toast({ title: "Error", body: String(e), logo: toastLogo });
    } finally {
      setUninstalling(null);
    }
  };

  const handleUninstall = (game: InstalledGame) => {
    showModal(
      <ConfirmActionModal
        title="Uninstall game"
        description={`Remove "${game.name}" (${formatSize(game.size)})? This action cannot be undone.`}
        confirmText="Remove"
        destructive
        onConfirm={() => doUninstall(game)}
      />
    );
  };

  if (!enabled) return null;

  return (
    <PanelSection title="Installed Games">
      {loading ? (
          <PanelSectionRow>
            <Field label="Loading...">
              <span style={{ opacity: 0.6 }}>...</span>
            </Field>
          </PanelSectionRow>
        ) : games.length === 0 ? (
          <PanelSectionRow>
            <Field
              label="No games"
              icon={<FaFolderOpen color={colors.capy} style={{ opacity: 0.5 }} />}
            >
              <span className="cd-text-disabled" style={{ fontSize: "0.85em" }}>
                Send games from the Hub
              </span>
            </Field>
          </PanelSectionRow>
      ) : (
        games.map((game) => (
          <PanelSectionRow key={game.name}>
            <Field
              label={game.name}
              description={formatSize(game.size)}
              icon={<FaGamepad color={colors.capy} />}
            >
              <Focusable
                className="cd-icon-btn"
                onClick={() => handleUninstall(game)}
                style={{ opacity: uninstalling === game.name ? 0.3 : 1 }}
              >
                <FaTrash size={14} color={colors.destructive} />
              </Focusable>
            </Field>
          </PanelSectionRow>
        ))
      )}
    </PanelSection>
  );
};

export default InstalledGames;
