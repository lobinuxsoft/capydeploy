/**
 * StatusPanel - Main panel showing connection status and controls.
 * Matches the Linux Agent UI with full status information.
 */

import {
  PanelSection,
  PanelSectionRow,
  ToggleField,
  Field,
  TextField,
  ButtonItem,
  Focusable,
} from "@decky/ui";
import { call, openFilePicker } from "@decky/api";
import { VFC, useState } from "react";
import {
  FaPlug,
  FaPlugCircleXmark,
  FaNetworkWired,
  FaComputer,
  FaFolder,
  FaFolderOpen,
  FaCircleInfo,
  FaPen,
  FaCheck,
  FaXmark,
  FaKey,
} from "react-icons/fa6";

// FileSelectionType enum from @decky/api
const FileSelectionType = {
  FILE: 0,
  FOLDER: 1,
} as const;

interface StatusPanelProps {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
  connected: boolean;
  hubName: string | null;
  pairingCode: string | null;
  agentName: string;
  platform: string;
  version: string;
  port: number;
  ip: string;
  installPath: string;
  onRefresh: () => void;
}

const StatusPanel: VFC<StatusPanelProps> = ({
  enabled,
  onEnabledChange,
  connected,
  hubName,
  pairingCode,
  agentName,
  platform,
  version,
  port,
  ip,
  installPath,
  onRefresh,
}) => {
  const [editingName, setEditingName] = useState(false);
  const [newName, setNewName] = useState(agentName);
  const [savingName, setSavingName] = useState(false);

  const handleSaveName = async () => {
    if (!newName.trim()) return;
    setSavingName(true);
    try {
      await call<[string], void>("set_agent_name", newName.trim());
      setEditingName(false);
      onRefresh();
    } catch (e) {
      console.error("Failed to save name:", e);
    } finally {
      setSavingName(false);
    }
  };

  const handleCancelEdit = () => {
    setEditingName(false);
    setNewName(agentName);
  };

  const handleSelectFolder = async () => {
    try {
      const result = await openFilePicker(
        FileSelectionType.FOLDER,
        installPath || "/home",
        false, // includeFiles
        true,  // includeFolders
      );
      if (result?.path) {
        await call<[string], void>("set_install_path", result.path);
        onRefresh();
      }
    } catch (e) {
      console.error("Failed to select folder:", e);
    }
  };

  const getPlatformDisplay = (p: string): string => {
    const platforms: Record<string, string> = {
      steamdeck: "Steam Deck",
      bazzite: "Bazzite",
      chimeraos: "ChimeraOS",
      linux: "Linux",
      windows: "Windows",
    };
    return platforms[p.toLowerCase()] || p;
  };

  return (
    <>
      {/* Main Toggle */}
      <PanelSection title="Estado">
        <PanelSectionRow>
          <ToggleField
            label="Activar CapyDeploy"
            description="Recibir juegos desde el Hub"
            checked={enabled}
            onChange={onEnabledChange}
          />
        </PanelSectionRow>

        {enabled && (
          <>
            {/* Connection Status */}
            <PanelSectionRow>
              <Field
                label="Conexion"
                icon={
                  connected ? (
                    <FaPlug color="#59bf40" />
                  ) : (
                    <FaPlugCircleXmark color="#bf4040" />
                  )
                }
              >
                <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span
                    style={{
                      color: connected ? "#59bf40" : "#bf4040",
                      fontWeight: "bold",
                    }}
                  >
                    {connected ? "Conectado" : "Esperando Hub..."}
                  </span>
                </Focusable>
              </Field>
            </PanelSectionRow>

            {/* Connected Hub */}
            {connected && hubName && (
              <PanelSectionRow>
                <Field label="Hub conectado">
                  <span style={{ color: "#59bf40" }}>{hubName}</span>
                </Field>
              </PanelSectionRow>
            )}

            {/* Pairing Code */}
            {pairingCode && (
              <PanelSectionRow>
                <Field
                  label="Codigo de emparejamiento"
                  description="Ingresa este codigo en el Hub"
                  icon={<FaKey color="#59bf40" />}
                >
                  <span
                    style={{
                      fontSize: "1.5em",
                      fontFamily: "monospace",
                      fontWeight: "bold",
                      letterSpacing: "0.3em",
                      color: "#59bf40",
                    }}
                  >
                    {pairingCode}
                  </span>
                </Field>
              </PanelSectionRow>
            )}
          </>
        )}
      </PanelSection>

      {/* Agent Info - Always visible */}
      <PanelSection title="Informacion del Agente">
        {/* Name - Editable */}
        <PanelSectionRow>
          {editingName ? (
            <Field label="Nombre" icon={<FaComputer />}>
              <Focusable style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <TextField
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  disabled={savingName}
                  style={{ flex: 1, minWidth: "100px" }}
                />
                <ButtonItem
                  layout="below"
                  onClick={handleSaveName}
                  disabled={savingName || !newName.trim()}
                >
                  <FaCheck color="#59bf40" />
                </ButtonItem>
                <ButtonItem
                  layout="below"
                  onClick={handleCancelEdit}
                  disabled={savingName}
                >
                  <FaXmark color="#bf4040" />
                </ButtonItem>
              </Focusable>
            </Field>
          ) : (
            <Field
              label="Nombre"
              icon={<FaComputer />}
              onClick={() => {
                setNewName(agentName);
                setEditingName(true);
              }}
            >
              <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span>{agentName}</span>
                <FaPen size={12} style={{ opacity: 0.5 }} />
              </Focusable>
            </Field>
          )}
        </PanelSectionRow>

        <PanelSectionRow>
          <Field label="Plataforma">
            <span>{getPlatformDisplay(platform)}</span>
          </Field>
        </PanelSectionRow>

        <PanelSectionRow>
          <Field label="Version" icon={<FaCircleInfo />}>
            <span style={{ fontFamily: "monospace" }}>{version}</span>
          </Field>
        </PanelSectionRow>

        {/* Install Path - Selectable */}
        <PanelSectionRow>
          <Field
            label="Ruta de instalacion"
            icon={<FaFolder />}
            onClick={handleSelectFolder}
          >
            <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "0.85em", opacity: 0.8 }}>{installPath}</span>
              <FaFolderOpen size={14} style={{ opacity: 0.5 }} />
            </Focusable>
          </Field>
        </PanelSectionRow>
      </PanelSection>

      {/* Network Info - Only when enabled */}
      {enabled && (
        <PanelSection title="Red">
          <PanelSectionRow>
            <Field label="Puerto" icon={<FaNetworkWired />}>
              <span style={{ fontFamily: "monospace" }}>{port}</span>
            </Field>
          </PanelSectionRow>

          <PanelSectionRow>
            <Field label="IP">
              <span style={{ fontFamily: "monospace" }}>{ip}</span>
            </Field>
          </PanelSectionRow>
        </PanelSection>
      )}

      {/* Capabilities */}
      <PanelSection title="Capacidades">
        <PanelSectionRow>
          <Field label="Subida de archivos">
            <span style={{ color: "#59bf40" }}>Si</span>
          </Field>
        </PanelSectionRow>
        <PanelSectionRow>
          <Field label="Shortcuts de Steam">
            <span style={{ color: "#59bf40" }}>Si</span>
          </Field>
        </PanelSectionRow>
        <PanelSectionRow>
          <Field label="Artwork de Steam">
            <span style={{ color: "#59bf40" }}>Si</span>
          </Field>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
};

export default StatusPanel;
