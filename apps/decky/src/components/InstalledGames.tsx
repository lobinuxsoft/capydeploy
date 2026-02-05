/**
 * InstalledGames - Shows games installed by CapyDeploy with uninstall option.
 */

import {
  PanelSection,
  PanelSectionRow,
  Field,
  ButtonItem,
  ConfirmModal,
  showModal,
} from "@decky/ui";
import { call, toaster } from "@decky/api";
import { VFC, useState, useEffect, useCallback } from "react";
import { FaGamepad, FaTrash, FaFolderOpen } from "react-icons/fa6";
import { colors } from "../styles/theme";

import mascotUrl from "../../assets/mascot.gif";

const toastLogo = (
  <img
    src={mascotUrl}
    style={{ width: "100%", height: "100%", borderRadius: "50%", objectFit: "cover" }}
  />
);

declare const SteamClient: {
  Apps: {
    RemoveShortcut: (appId: number) => void;
  };
};

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

  const handleUninstall = (game: InstalledGame) => {
    showModal(
      <ConfirmModal
        strTitle="Desinstalar juego"
        strDescription={`¿Eliminar "${game.name}" (${formatSize(game.size)})? Esta accion no se puede deshacer.`}
        strOKButtonText="Eliminar"
        strCancelButtonText="Cancelar"
        onOK={async () => {
          setUninstalling(game.name);
          try {
            const result = await call<[string], number | boolean>("uninstall_game", game.name);
            if (result) {
              // result is appId (number) or true (boolean) — remove Steam shortcut
              if (typeof result === "number" && result > 0) {
                try {
                  SteamClient.Apps.RemoveShortcut(result);
                } catch (e) {
                  console.error("Failed to remove shortcut:", e);
                }
              }
              setGames(games.filter((g) => g.name !== game.name));
              toaster.toast({
                title: "Juego eliminado",
                body: game.name,
                logo: toastLogo,
              });
            } else {
              toaster.toast({
                title: "Error",
                body: `No se pudo eliminar ${game.name}`,
                logo: toastLogo,
              });
            }
          } catch (e) {
            console.error("Failed to uninstall:", e);
            toaster.toast({
              title: "Error",
              body: String(e),
              logo: toastLogo,
            });
          } finally {
            setUninstalling(null);
          }
        }}
      />
    );
  };

  if (!enabled) return null;

  return (
    <PanelSection title="Juegos Instalados">
      {loading ? (
        <PanelSectionRow>
          <Field label="Cargando...">
            <span style={{ opacity: 0.6 }}>...</span>
          </Field>
        </PanelSectionRow>
      ) : games.length === 0 ? (
        <PanelSectionRow>
          <Field
            label="Sin juegos"
            icon={<FaFolderOpen style={{ opacity: 0.5 }} />}
          >
            <span style={{ fontSize: "0.85em", opacity: 0.6 }}>
              Envia juegos desde el Hub
            </span>
          </Field>
        </PanelSectionRow>
      ) : (
        games.map((game) => (
          <PanelSectionRow key={game.name}>
            <Field
              label={game.name}
              description={formatSize(game.size)}
              icon={<FaGamepad />}
            >
              <ButtonItem
                layout="below"
                onClick={() => handleUninstall(game)}
                disabled={uninstalling === game.name}
              >
                <FaTrash
                  style={{
                    color: uninstalling === game.name ? colors.disabled : colors.destructive,
                  }}
                />
              </ButtonItem>
            </Field>
          </PanelSectionRow>
        ))
      )}
    </PanelSection>
  );
};

export default InstalledGames;
