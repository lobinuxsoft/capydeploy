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
} from "@decky/ui";
import { VFC } from "react";
import {
  FaPlug,
  FaPlugCircleXmark,
  FaNetworkWired,
  FaComputer,
  FaFolder,
  FaCircleInfo
} from "react-icons/fa6";

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
}) => {
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
        <PanelSectionRow>
          <Field label="Nombre" icon={<FaComputer />}>
            <span>{agentName}</span>
          </Field>
        </PanelSectionRow>

        <PanelSectionRow>
          <Field label="Plataforma">
            <span style={{ textTransform: "capitalize" }}>{platform}</span>
          </Field>
        </PanelSectionRow>

        <PanelSectionRow>
          <Field label="Version" icon={<FaCircleInfo />}>
            <span style={{ fontFamily: "monospace" }}>{version}</span>
          </Field>
        </PanelSectionRow>

        <PanelSectionRow>
          <Field label="Ruta de instalacion" icon={<FaFolder />}>
            <span style={{ fontSize: "0.85em", opacity: 0.8 }}>{installPath}</span>
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
    </>
  );
};

export default StatusPanel;
