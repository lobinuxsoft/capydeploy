/**
 * StatusPanel - Main panel showing connection status and controls.
 * Matches the Linux Agent UI with full status information.
 */

import {
  PanelSection,
  PanelSectionRow,
  ToggleField,
  Field,
  Focusable,
  showModal,
} from "@decky/ui";
import { call, openFilePicker } from "@decky/api";
import { VFC } from "react";
import {
  FaPlug,
  FaPlugCircleXmark,
  FaNetworkWired,
  FaComputer,
  FaFolder,
  FaFolderOpen,
  FaCircleInfo,
  FaPen,
  FaKey,
  FaChevronDown,
  FaChevronRight,
} from "react-icons/fa6";
import { colors } from "../styles/theme";
import { usePanelState } from "../hooks/usePanelState";
import NameEditModal from "./NameEditModal";

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
  // Collapsible section states (persisted across panel close/open)
  const [statusExpanded, toggleStatus] = usePanelState("status");
  const [infoExpanded, toggleInfo] = usePanelState("info");
  const [networkExpanded, toggleNetwork] = usePanelState("network");
  const handleEditName = () => {
    showModal(<NameEditModal currentName={agentName} onSaved={onRefresh} />);
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
        <div className="cd-section-title" onClick={toggleStatus}>
          {statusExpanded ? <FaChevronDown size={10} color={colors.primary} /> : <FaChevronRight size={10} color={colors.disabled} />}
          Status
        </div>
        {statusExpanded && (
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
        )}
      </div>

      <div className="cd-section">
        <div className="cd-section-title" onClick={toggleInfo}>
          {infoExpanded ? <FaChevronDown size={10} color={colors.primary} /> : <FaChevronRight size={10} color={colors.disabled} />}
          Agent Info
        </div>
        {infoExpanded && (
          <PanelSection>
          <PanelSectionRow>
            <Field
              label="Name"
              icon={<FaComputer color={colors.capy} />}
              onClick={handleEditName}
            >
              <Focusable style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span className="cd-value">{agentName}</span>
                <FaPen size={12} style={{ opacity: 0.5 }} />
              </Focusable>
            </Field>
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
        )}
      </div>

      {enabled && (
        <div className="cd-section">
          <div className="cd-section-title" onClick={toggleNetwork}>
            {networkExpanded ? <FaChevronDown size={10} color={colors.primary} /> : <FaChevronRight size={10} color={colors.disabled} />}
            Network
          </div>
          {networkExpanded && (
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
          )}
        </div>
      )}

    </>
  );
};

export default StatusPanel;
