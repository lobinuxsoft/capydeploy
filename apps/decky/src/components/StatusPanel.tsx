/**
 * StatusPanel - Main panel showing connection status and controls.
 */

import {
  PanelSection,
  PanelSectionRow,
  ToggleField,
  Field,
  Focusable,
} from "@decky/ui";
import { VFC } from "react";
import { FaPlug, FaPlugCircleXmark } from "react-icons/fa6";

interface StatusPanelProps {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
  connected: boolean;
  hubName: string | null;
  pairingCode: string | null;
}

const StatusPanel: VFC<StatusPanelProps> = ({
  enabled,
  onEnabledChange,
  connected,
  hubName,
  pairingCode,
}) => {
  return (
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

          {connected && hubName && (
            <PanelSectionRow>
              <Field label="Hub conectado">
                <span>{hubName}</span>
              </Field>
            </PanelSectionRow>
          )}

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
  );
};

export default StatusPanel;
