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
import { colors } from "../styles/theme";

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
      <div className="cd-section">
        <div className="cd-section-title">Status</div>
        <PanelSection>
          <PanelSectionRow>
            <ToggleField
              label="Enable CapyDeploy"
              description="Receive games from the Hub"
              checked={enabled}
              onChange={onEnabledChange}
            />
          </PanelSectionRow>

          {enabled && (
            <>
              <PanelSectionRow>
                <Field
                  label="Connection"
                  icon={
                    connected ? (
                      <FaPlug color={colors.capy} />
                    ) : (
                      <FaPlugCircleXmark color={colors.destructive} />
                    )
                  }
                >
                  <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                    <span className={connected ? "cd-status-connected" : "cd-status-disconnected"}>
                      {connected && <span className="cd-pulse" />}
                      {connected ? "Connected" : "Waiting for Hub..."}
                    </span>
                  </Focusable>
                </Field>
              </PanelSectionRow>

              {connected && hubName && (
                <PanelSectionRow>
                  <Field label="Connected Hub">
                    <span className="cd-text-primary">{hubName}</span>
                  </Field>
                </PanelSectionRow>
              )}

              {pairingCode && (
                <PanelSectionRow>
                  <Field
                    label="Pairing code"
                    description="Enter this code in the Hub"
                    icon={<FaKey color={colors.capy} />}
                  >
                    <span className="cd-pairing-code">
                      {pairingCode}
                    </span>
                  </Field>
                </PanelSectionRow>
              )}
            </>
          )}
        </PanelSection>
      </div>

      <div className="cd-section">
        <div className="cd-section-title">Agent Info</div>
        <PanelSection>
          <PanelSectionRow>
            {editingName ? (
              <Focusable style={{ display: "flex", alignItems: "center", gap: "8px", padding: "8px 16px" }}>
                <FaComputer color={colors.capy} style={{ flexShrink: 0 }} />
                <TextField
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  disabled={savingName}
                  style={{ flex: 1, minWidth: 0 }}
                />
                <Focusable
                  className="cd-icon-btn"
                  onClick={handleSaveName}
                  style={{ opacity: savingName || !newName.trim() ? 0.3 : 1 }}
                >
                  <FaCheck size={14} color={colors.primary} />
                </Focusable>
                <Focusable
                  className="cd-icon-btn"
                  onClick={handleCancelEdit}
                  style={{ opacity: savingName ? 0.3 : 1 }}
                >
                  <FaXmark size={14} color={colors.destructive} />
                </Focusable>
              </Focusable>
            ) : (
              <Field
                label="Name"
                icon={<FaComputer color={colors.capy} />}
                onClick={() => {
                  setNewName(agentName);
                  setEditingName(true);
                }}
              >
                <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span className="cd-value">{agentName}</span>
                  <FaPen size={12} style={{ opacity: 0.5 }} />
                </Focusable>
              </Field>
            )}
          </PanelSectionRow>

          <PanelSectionRow>
            <Field label="Platform">
              <span className="cd-value">{getPlatformDisplay(platform)}</span>
            </Field>
          </PanelSectionRow>

          <PanelSectionRow>
            <Field label="Version" icon={<FaCircleInfo color={colors.capy} />}>
              <span className="cd-mono">{version}</span>
            </Field>
          </PanelSectionRow>

          <PanelSectionRow>
            <Field
              label="Install path"
              icon={<FaFolder color={colors.capy} />}
              onClick={handleSelectFolder}
            >
              <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span className="cd-mono" style={{ fontSize: "0.85em" }}>{installPath}</span>
                <FaFolderOpen size={14} style={{ opacity: 0.5 }} />
              </Focusable>
            </Field>
          </PanelSectionRow>
        </PanelSection>
      </div>

      {enabled && (
        <div className="cd-section">
          <div className="cd-section-title">Network</div>
          <PanelSection>
            <PanelSectionRow>
              <Field label="Port" icon={<FaNetworkWired color={colors.capy} />}>
                <span className="cd-mono">{port}</span>
              </Field>
            </PanelSectionRow>

            <PanelSectionRow>
              <Field label="IP">
                <span className="cd-mono">{ip}</span>
              </Field>
            </PanelSectionRow>
          </PanelSection>
        </div>
      )}

      <div className="cd-section">
        <div className="cd-section-title">Capabilities</div>
        <PanelSection>
          <PanelSectionRow>
            <Field label="File upload">
              <span className="cd-text-primary">Yes</span>
            </Field>
          </PanelSectionRow>
          <PanelSectionRow>
            <Field label="Steam Shortcuts">
              <span className="cd-text-primary">Yes</span>
            </Field>
          </PanelSectionRow>
          <PanelSectionRow>
            <Field label="Steam Artwork">
              <span className="cd-text-primary">Yes</span>
            </Field>
          </PanelSectionRow>
        </PanelSection>
      </div>
    </>
  );
};

export default StatusPanel;
