/**
 * CapyDeploy Decky Theme
 * Centralized color palette and CSS for the QAM panel.
 * Uses inline <style> JSX + Steam class overrides via quickAccessControlsClasses.
 */

import { quickAccessControlsClasses, gamepadDialogClasses } from "@decky/ui";

// ── Color constants (for react-icons inline `color` prop) ──────────────────

export const colors = {
  primary: "#06b6d4",
  primaryMid: "rgba(6, 182, 212, 0.35)",
  primaryHalf: "rgba(6, 182, 212, 0.5)",
  capy: "#f97316",
  capyLight: "#fb923c",
  destructive: "#dc2626",
  disabled: "#94a3b8",
  foreground: "#f1f5f9",
} as const;

// Brand gradient used across Hub, Agent and Docs
const GRADIENT = "linear-gradient(90deg, #f97316 0%, #fb923c 40%, #06b6d4 100%)";

// ── CSS builder (called at render time so Steam classes are resolved) ──────

export function getThemeCSS(): string {
  const secTitle = quickAccessControlsClasses?.PanelSectionTitle;
  const fieldLabel = gamepadDialogClasses?.FieldLabel;
  const fieldDesc = gamepadDialogClasses?.FieldDescription;
  const sepStd = gamepadDialogClasses?.WithBottomSeparatorStandard;
  const fieldChildren = gamepadDialogClasses?.FieldChildrenWithIcon;
  const fieldChildren2 = gamepadDialogClasses?.FieldChildrenInner;

  return `
  /* ── Steam native overrides (scoped) ──────────────────── */

  ${secTitle ? `
  #capydeploy-wrap .${secTitle} {
    background: ${GRADIENT} !important;
    -webkit-background-clip: text !important;
    -webkit-text-fill-color: transparent !important;
    background-clip: text !important;
  }
  ` : ""}

  /* ── Custom section title (replaces PanelSection title) ── */

  .cd-section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    text-transform: uppercase;
    font-size: 0.75em;
    font-weight: 600;
    letter-spacing: 0.1em;
    padding: 12px 16px 4px 16px;
    background: ${GRADIENT};
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    cursor: pointer;
    user-select: none;
    transition: opacity 0.15s ease;
  }

  .cd-section-title:hover {
    opacity: 0.8;
  }

  .cd-section-title svg {
    flex-shrink: 0;
    filter: drop-shadow(0 0 3px ${colors.primaryMid});
  }

  ${fieldLabel ? `
  #capydeploy-wrap .${fieldLabel} {
    text-shadow: 0 0 12px rgba(241, 245, 249, 0.15) !important;
  }
  ` : ""}

  ${fieldDesc ? `
  #capydeploy-wrap .${fieldDesc} {
    color: ${colors.disabled} !important;
  }
  ` : ""}

  ${sepStd ? `
  #capydeploy-wrap .${sepStd}::after {
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(249, 115, 22, 0.3) 30%,
      rgba(6, 182, 212, 0.3) 70%,
      transparent 100%
    ) !important;
    height: 1px !important;
    opacity: 0.8 !important;
  }
  ` : ""}

  ${fieldChildren ? `
  #capydeploy-wrap .${fieldChildren} svg {
    filter: drop-shadow(0 0 3px ${colors.primaryMid});
  }
  ` : ""}

  ${fieldChildren2 ? `
  #capydeploy-wrap .${fieldChildren2} {
    color: ${colors.foreground};
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
      rgba(249, 115, 22, 0.04) 50%,
      rgba(6, 182, 212, 0.08) 100%
    );
    border: 1px solid rgba(6, 182, 212, 0.2);
    border-radius: 12px;
    position: relative;
    overflow: hidden;
    box-shadow:
      0 2px 12px rgba(6, 182, 212, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }

  .cd-header::before {
    content: "";
    position: absolute;
    top: -40%;
    right: -15%;
    width: 120px;
    height: 120px;
    background: radial-gradient(circle, rgba(249, 115, 22, 0.1) 0%, transparent 70%);
    pointer-events: none;
  }

  /* ── Mascot: circle + radial fade + glow aura + ring ── */

  .cd-mascot-wrap {
    position: relative;
    width: 68px;
    height: 68px;
    flex-shrink: 0;
  }

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
    background: ${GRADIENT};
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    letter-spacing: 0.03em;
  }

  .cd-subtitle {
    font-size: 0.8em;
    color: ${colors.disabled};
    margin-top: 3px;
    letter-spacing: 0.02em;
  }

  /* ── Accent bar under header ─────────────────────────────── */

  .cd-header::after {
    content: "";
    position: absolute;
    bottom: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: ${GRADIENT};
    opacity: 0.4;
  }

  /* ── Section glass panels ─────────────────────────────────── */

  .cd-section {
    margin: 4px 0;
    border-radius: 10px;
    position: relative;
    overflow: hidden;
    background: linear-gradient(
      135deg,
      rgba(6, 182, 212, 0.12) 0%,
      rgba(249, 115, 22, 0.04) 50%,
      rgba(6, 182, 212, 0.08) 100%
    );
    border: 1px solid rgba(6, 182, 212, 0.2);
    box-shadow:
      0 2px 12px rgba(6, 182, 212, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
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
    color: ${colors.primary};
    text-shadow: 0 0 6px ${colors.primaryMid};
    font-weight: 500;
  }

  .cd-text-primary {
    color: ${colors.primary};
    font-weight: bold;
    text-shadow: 0 0 8px ${colors.primaryMid};
  }

  .cd-text-capy {
    color: ${colors.capy};
    text-shadow: 0 0 6px rgba(249, 115, 22, 0.3);
  }

  .cd-text-destructive {
    color: ${colors.destructive};
    text-shadow: 0 0 4px rgba(220, 38, 38, 0.3);
  }

  .cd-text-disabled {
    color: ${colors.disabled};
  }

  /* ── Compact icon buttons (trash, etc.) ───────────────────── */

  .cd-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .cd-icon-btn:hover,
  .cd-icon-btn.gpfocus {
    background: rgba(255, 255, 255, 0.12);
    border-color: rgba(255, 255, 255, 0.2);
  }

  /* ── Field value styling (for inline spans) ────────────── */

  .cd-value {
    color: ${colors.foreground};
    text-shadow: 0 0 8px rgba(241, 245, 249, 0.12);
  }

  /* ── Progress modal ─────────────────────────────────────── */

  .cd-modal-progress {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px;
    gap: 12px;
    min-width: 300px;
  }

  .cd-modal-mascot {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    -webkit-mask-image: radial-gradient(circle, #000 60%, transparent 78%);
    mask-image: radial-gradient(circle, #000 60%, transparent 78%);
  }

  .cd-modal-title {
    font-weight: bold;
    font-size: 1.2em;
    background: ${GRADIENT};
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    text-align: center;
  }

  .cd-modal-game {
    font-size: 1.1em;
    color: ${colors.foreground};
    text-align: center;
  }

  .cd-modal-status {
    font-size: 0.9em;
    color: ${colors.disabled};
    text-align: center;
  }

  .cd-modal-status-error {
    color: ${colors.destructive};
  }

  .cd-modal-status-done {
    color: ${colors.primary};
    text-shadow: 0 0 8px ${colors.primaryMid};
  }

  .cd-progress-bar {
    width: 100%;
    height: 8px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    overflow: hidden;
    margin-top: 4px;
  }

  .cd-progress-fill {
    height: 100%;
    background: ${GRADIENT};
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .cd-progress-pct {
    font-family: monospace;
    font-size: 0.85em;
    color: ${colors.disabled};
  }

  .cd-progress-bytes {
    font-size: 0.8em;
    color: ${colors.disabled};
  }

  .cd-modal-check {
    font-size: 2em;
    color: ${colors.primary};
    text-shadow: 0 0 16px ${colors.primaryHalf};
    animation: cd-check-pop 0.4s ease-out;
  }

  @keyframes cd-check-pop {
    0% { transform: scale(0); opacity: 0; }
    60% { transform: scale(1.3); }
    100% { transform: scale(1); opacity: 1; }
  }
  `;
}

// ── Progress modal CSS (self-contained for modal context) ──────────────────

export function getModalCSS(): string {
  return `
  /* ── Glass backdrop for ModalRoot ────────────────────────── */

  .cd-modal-progress {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 28px 24px;
    gap: 12px;
    min-width: 320px;
    background: linear-gradient(
      135deg,
      rgba(6, 182, 212, 0.12) 0%,
      rgba(15, 23, 42, 0.85) 40%,
      rgba(249, 115, 22, 0.06) 100%
    );
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(6, 182, 212, 0.2);
    border-radius: 16px;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.4),
      0 0 24px rgba(6, 182, 212, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }
  .cd-modal-mascot {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    -webkit-mask-image: radial-gradient(circle, #000 60%, transparent 78%);
    mask-image: radial-gradient(circle, #000 60%, transparent 78%);
  }
  .cd-modal-title {
    font-weight: bold;
    font-size: 1.2em;
    background: ${GRADIENT};
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    text-align: center;
  }
  .cd-modal-game {
    font-size: 1.1em;
    color: ${colors.foreground};
    text-align: center;
  }
  .cd-modal-status {
    font-size: 0.9em;
    color: ${colors.disabled};
    text-align: center;
  }
  .cd-modal-status-error { color: ${colors.destructive}; }
  .cd-modal-status-done {
    color: ${colors.primary};
    text-shadow: 0 0 8px ${colors.primaryMid};
  }
  .cd-progress-bar {
    width: 100%;
    height: 8px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    overflow: hidden;
    margin-top: 4px;
  }
  .cd-progress-fill {
    height: 100%;
    background: ${GRADIENT};
    border-radius: 4px;
    transition: width 0.3s ease;
  }
  .cd-progress-pct {
    font-family: monospace;
    font-size: 0.85em;
    color: ${colors.disabled};
  }
  .cd-progress-bytes {
    font-size: 0.8em;
    color: ${colors.disabled};
  }
  .cd-modal-check {
    font-size: 2em;
    color: ${colors.primary};
    text-shadow: 0 0 16px ${colors.primaryHalf};
    animation: cd-check-pop 0.4s ease-out;
  }
  @keyframes cd-check-pop {
    0% { transform: scale(0); opacity: 0; }
    60% { transform: scale(1.3); }
    100% { transform: scale(1); opacity: 1; }
  }

  /* ── Modal subtitle ─────────────────────────────────────── */

  .cd-modal-subtitle {
    font-size: 0.9em;
    color: ${colors.disabled};
    text-align: center;
  }

  /* ── Pairing code display ───────────────────────────────── */

  .cd-modal-code {
    font-size: 2.2em;
    font-family: monospace;
    font-weight: bold;
    letter-spacing: 0.35em;
    text-align: center;
    padding: 16px 24px;
    margin: 8px 0;
    background: ${GRADIENT};
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    border: 1px solid rgba(6, 182, 212, 0.25);
    border-radius: 10px;
    position: relative;
  }

  .cd-modal-code::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 10px;
    background: linear-gradient(
      135deg,
      rgba(6, 182, 212, 0.08) 0%,
      rgba(249, 115, 22, 0.04) 50%,
      rgba(6, 182, 212, 0.06) 100%
    );
    z-index: -1;
  }

  /* ── Modal input wrapper ────────────────────────────────── */

  .cd-modal-input-wrap {
    width: 100%;
    margin: 4px 0;
  }

  .cd-modal-input-wrap input {
    width: 100% !important;
    text-align: center;
    font-size: 1.1em !important;
  }

  /* ── Modal action buttons ───────────────────────────────── */

  .cd-modal-actions {
    display: flex;
    gap: 10px;
    margin-top: 8px;
    width: 100%;
    justify-content: center;
  }

  .cd-modal-btn {
    padding: 10px 28px;
    border-radius: 8px;
    font-size: 0.95em;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.15s ease;
  }

  .cd-modal-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .cd-modal-btn-primary {
    background: linear-gradient(135deg, ${colors.capy}, ${colors.primary});
    color: #fff;
    border-color: rgba(6, 182, 212, 0.3);
  }

  .cd-modal-btn-primary:hover:not(:disabled) {
    box-shadow: 0 0 16px ${colors.primaryMid};
  }

  .cd-modal-btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: ${colors.disabled};
    border-color: rgba(255, 255, 255, 0.12);
  }

  .cd-modal-btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
    color: ${colors.foreground};
  }

  .cd-modal-btn-danger {
    background: linear-gradient(135deg, ${colors.destructive}, #991b1b);
    color: #fff;
    border-color: rgba(220, 38, 38, 0.4);
  }

  .cd-modal-btn-danger:hover:not(:disabled) {
    box-shadow: 0 0 16px rgba(220, 38, 38, 0.35);
  }
  `;
}
