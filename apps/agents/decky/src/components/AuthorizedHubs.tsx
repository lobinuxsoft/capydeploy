/**
 * AuthorizedHubs - Shows list of authorized hubs with revoke option.
 */

import {
  PanelSection,
  PanelSectionRow,
  Field,
  Focusable,
} from "@decky/ui";
import { call } from "@decky/api";
import { VFC, useState, useEffect, useCallback } from "react";
import { FaShieldHalved, FaTrash, FaComputer, FaChevronDown, FaChevronRight } from "react-icons/fa6";
import { colors } from "../styles/theme";
import { usePanelState } from "../hooks/usePanelState";

interface AuthorizedHub {
  id: string;
  name: string;
  platform?: string;
  pairedAt: number;
}

const getPlatformIcon = (platform?: string): string => {
  switch (platform) {
    case "windows": return "🪟";
    case "darwin": return "🍎";
    case "linux": return "🐧";
    default: return "💻";
  }
};

const getPlatformName = (platform?: string): string => {
  switch (platform) {
    case "windows": return "Windows";
    case "darwin": return "macOS";
    case "linux": return "Linux";
    default: return platform || "Unknown";
  }
};

interface AuthorizedHubsProps {
  enabled: boolean;
}

const AuthorizedHubs: VFC<AuthorizedHubsProps> = ({ enabled }) => {
  const [hubs, setHubs] = useState<AuthorizedHub[]>([]);
  const [loading, setLoading] = useState(true);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [expanded, toggleExpanded] = usePanelState("authorizedHubs");

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
    if (!timestamp) return "Unknown";
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString("en", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  };

  if (!enabled) return null;

  return (
    <div className="cd-section">
      <div className="cd-section-title" onClick={toggleExpanded}>
        {expanded ? <FaChevronDown size={10} color={colors.primary} /> : <FaChevronRight size={10} color={colors.disabled} />}
        Authorized Hubs
      </div>
      {expanded && (
        <PanelSection>
          {loading ? (
          <PanelSectionRow>
            <Field label="Loading...">
              <span style={{ opacity: 0.6 }}>...</span>
            </Field>
          </PanelSectionRow>
        ) : hubs.length === 0 ? (
          <PanelSectionRow>
            <Field
              label="No hubs"
              icon={<FaShieldHalved color={colors.capy} style={{ opacity: 0.5 }} />}
            >
              <span className="cd-text-disabled" style={{ fontSize: "0.85em" }}>
                Connect a Hub to pair
              </span>
            </Field>
          </PanelSectionRow>
        ) : (
          hubs.map((hub) => (
            <PanelSectionRow key={hub.id}>
              <Field
                label={
                  <span>
                    {hub.name}
                    {hub.platform && (
                      <span title={getPlatformName(hub.platform)} style={{ marginLeft: "6px" }}>
                        {getPlatformIcon(hub.platform)}
                      </span>
                    )}
                  </span>
                }
                description={`Paired: ${formatDate(hub.pairedAt)}`}
                icon={<FaComputer color={colors.capy} />}
              >
                <Focusable
                  className="cd-icon-btn"
                  onClick={() => handleRevoke(hub.id)}
                  style={{ opacity: revoking === hub.id ? 0.3 : 1 }}
                >
                  <FaTrash size={14} color={colors.destructive} />
                </Focusable>
              </Field>
            </PanelSectionRow>
            ))
          )}
        </PanelSection>
      )}
    </div>
  );
};

export default AuthorizedHubs;
