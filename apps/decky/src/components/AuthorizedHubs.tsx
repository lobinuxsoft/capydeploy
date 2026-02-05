/**
 * AuthorizedHubs - Shows list of authorized hubs with revoke option.
 */

import {
  PanelSection,
  PanelSectionRow,
  Field,
  ButtonItem,
} from "@decky/ui";
import { call } from "@decky/api";
import { VFC, useState, useEffect, useCallback } from "react";
import { FaShieldHalved, FaTrash, FaComputer } from "react-icons/fa6";
import { colors } from "../styles/theme";

interface AuthorizedHub {
  id: string;
  name: string;
  pairedAt: number;
}

interface AuthorizedHubsProps {
  enabled: boolean;
}

const AuthorizedHubs: VFC<AuthorizedHubsProps> = ({ enabled }) => {
  const [hubs, setHubs] = useState<AuthorizedHub[]>([]);
  const [loading, setLoading] = useState(true);
  const [revoking, setRevoking] = useState<string | null>(null);

  const loadHubs = useCallback(async () => {
    try {
      const result = await call<[], AuthorizedHub[]>("get_authorized_hubs");
      setHubs(result || []);
    } catch (e) {
      console.error("Failed to load hubs:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      loadHubs();
    }
  }, [enabled, loadHubs]);

  const handleRevoke = async (hubId: string) => {
    setRevoking(hubId);
    try {
      await call<[string], boolean>("revoke_hub", hubId);
      setHubs(hubs.filter((h) => h.id !== hubId));
    } catch (e) {
      console.error("Failed to revoke hub:", e);
    } finally {
      setRevoking(null);
    }
  };

  const formatDate = (timestamp: number): string => {
    if (!timestamp) return "Desconocido";
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString("es", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  };

  if (!enabled) return null;

  return (
    <PanelSection title="Hubs Autorizados">
      {loading ? (
        <PanelSectionRow>
          <Field label="Cargando...">
            <span style={{ opacity: 0.6 }}>...</span>
          </Field>
        </PanelSectionRow>
      ) : hubs.length === 0 ? (
        <PanelSectionRow>
          <Field
            label="Sin hubs"
            icon={<FaShieldHalved style={{ opacity: 0.5 }} />}
          >
            <span style={{ fontSize: "0.85em", opacity: 0.6 }}>
              Conecta un Hub para emparejar
            </span>
          </Field>
        </PanelSectionRow>
      ) : (
        hubs.map((hub) => (
          <PanelSectionRow key={hub.id}>
            <Field
              label={hub.name}
              description={`Emparejado: ${formatDate(hub.pairedAt)}`}
              icon={<FaComputer />}
            >
              <ButtonItem
                layout="below"
                onClick={() => handleRevoke(hub.id)}
                disabled={revoking === hub.id}
              >
                <FaTrash
                  style={{
                    color: revoking === hub.id ? colors.disabled : colors.destructive,
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

export default AuthorizedHubs;
