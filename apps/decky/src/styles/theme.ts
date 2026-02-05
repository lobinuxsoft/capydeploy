/**
 * CapyDeploy Decky Theme
 * Centralized color palette and CSS for the QAM panel.
 * Uses inline <style> JSX + Steam class overrides via quickAccessControlsClasses.
 */

import { quickAccessControlsClasses } from "@decky/ui";

// ── Color constants (for react-icons inline `color` prop) ──────────────────

export const colors = {
  primary: "#06b6d4",
  primaryMid: "rgba(6, 182, 212, 0.35)",
  primaryHalf: "rgba(6, 182, 212, 0.5)",
  destructive: "#dc2626",
  disabled: "#94a3b8",
  foreground: "#f1f5f9",
} as const;

// ── CSS builder (called at render time so Steam classes are resolved) ──────

export function getThemeCSS(): string {
  const secTitle = quickAccessControlsClasses?.PanelSectionTitle;

  return `
  /* ── Steam native overrides (scoped) ──────────────────── */

  ${secTitle ? `
  #capydeploy-wrap .${secTitle} {
    background: linear-gradient(90deg, ${colors.primary}, #67e8f9) !important;
    -webkit-background-clip: text !important;
    -webkit-text-fill-color: transparent !important;
    background-clip: text !important;
  }
  ` : ""}

  /* ── Header glass panel ───────────────────────────────── */

  .cd-header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    margin: 0 0 8px 0;
    background: linear-gradient(
      135deg,
      rgba(6, 182, 212, 0.12) 0%,
      rgba(6, 182, 212, 0.03) 100%
    );
    border: 1px solid rgba(6, 182, 212, 0.18);
    border-radius: 12px;
    position: relative;
    overflow: hidden;
  }

  /* Ambient glow in top-right corner */
  .cd-header::before {
    content: "";
    position: absolute;
    top: -40%;
    right: -15%;
    width: 100px;
    height: 100px;
    background: radial-gradient(circle, rgba(6, 182, 212, 0.15) 0%, transparent 70%);
    pointer-events: none;
  }

  /* ── Mascot: circle + radial fade + glow aura + ring ── */

  .cd-mascot-wrap {
    position: relative;
    width: 68px;
    height: 68px;
    flex-shrink: 0;
  }

  /* Glow aura behind mascot */
  .cd-mascot-wrap::before {
    content: "";
    position: absolute;
    inset: -12px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(6, 182, 212, 0.25) 0%, transparent 70%);
    z-index: 0;
    animation: cd-aura-pulse 3s ease-in-out infinite;
  }

  @keyframes cd-aura-pulse {
    0%, 100% { opacity: 0.6; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.08); }
  }

  .cd-mascot-wrap img {
    position: relative;
    z-index: 1;
    width: 68px;
    height: 68px;
    border-radius: 50%;
    object-fit: cover;
    -webkit-mask-image: radial-gradient(circle, #000 58%, transparent 72%);
    mask-image: radial-gradient(circle, #000 58%, transparent 72%);
  }

  /* Animated conic ring */
  .cd-mascot-wrap::after {
    content: "";
    position: absolute;
    inset: -4px;
    z-index: 2;
    border-radius: 50%;
    background: conic-gradient(
      from 0deg,
      ${colors.primary},
      transparent 30%,
      ${colors.primary} 50%,
      transparent 80%,
      ${colors.primary}
    );
    -webkit-mask: radial-gradient(
      farthest-side,
      transparent calc(100% - 2.5px),
      #000 calc(100% - 1.5px)
    );
    mask: radial-gradient(
      farthest-side,
      transparent calc(100% - 2.5px),
      #000 calc(100% - 1.5px)
    );
    animation: cd-ring-spin 4s linear infinite;
    filter: drop-shadow(0 0 4px ${colors.primaryHalf});
  }

  @keyframes cd-ring-spin {
    to { transform: rotate(360deg); }
  }

  /* ── Title ─────────────────────────────────────────────── */

  .cd-title {
    font-weight: bold;
    font-size: 1.3em;
    background: linear-gradient(135deg, #67e8f9 0%, ${colors.primary} 45%, #a5f3fc 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    letter-spacing: 0.03em;
  }

  .cd-subtitle {
    font-size: 0.8em;
    color: ${colors.disabled};
    margin-top: 3px;
  }

  /* ── Status indicators ──────────────────────────────────── */

  .cd-status-connected {
    color: ${colors.primary};
    font-weight: bold;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    text-shadow: 0 0 8px ${colors.primaryMid};
  }

  .cd-status-disconnected {
    color: ${colors.destructive};
    font-weight: bold;
  }

  .cd-pulse {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: ${colors.primary};
    box-shadow: 0 0 6px ${colors.primary};
    animation: cd-pulse-anim 2s ease-in-out infinite;
  }

  @keyframes cd-pulse-anim {
    0%, 100% {
      opacity: 1;
      box-shadow: 0 0 4px ${colors.primaryHalf};
    }
    50% {
      opacity: 0.4;
      box-shadow: 0 0 12px ${colors.primary}, 0 0 20px ${colors.primaryMid};
    }
  }

  /* ── Pairing code ───────────────────────────────────────── */

  .cd-pairing-code {
    font-size: 1.5em;
    font-family: monospace;
    font-weight: bold;
    letter-spacing: 0.3em;
    color: ${colors.primary};
    text-shadow: 0 0 10px ${colors.primaryMid};
  }

  /* ── Utility classes ────────────────────────────────────── */

  .cd-mono {
    font-family: monospace;
    color: ${colors.foreground};
  }

  .cd-text-primary {
    color: ${colors.primary};
    text-shadow: 0 0 6px ${colors.primaryMid};
  }

  .cd-text-destructive {
    color: ${colors.destructive};
  }

  .cd-text-disabled {
    color: ${colors.disabled};
  }
  `;
}
