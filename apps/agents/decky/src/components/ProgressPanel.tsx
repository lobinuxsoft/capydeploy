/**
 * ProgressPanel - Shows current transfer progress (QAM inline + modal popup).
 */

import {
  PanelSection,
  PanelSectionRow,
  Field,
  ProgressBarWithInfo,
  ModalRoot,
} from "@decky/ui";
import { VFC, useState, useEffect } from "react";
import type { OperationEvent, UploadProgress } from "../types";
import { colors, getModalCSS } from "../styles/theme";

import mascotUrl from "../../assets/mascot.gif";

// ── Shared progress state (subscriber pattern for modal live updates) ──────

export const progressState = {
  operation: null as OperationEvent | null,
  progress: null as UploadProgress | null,
  _listeners: new Set<() => void>(),

  update(op: OperationEvent | null, prog: UploadProgress | null) {
    this.operation = op;
    this.progress = prog;
    this._listeners.forEach((l) => l());
  },

  subscribe(fn: () => void) {
    this._listeners.add(fn);
    return () => { this._listeners.delete(fn); };
  },
};

// ── Progress modal content (rendered inside showModal) ─────────────────────

export const ProgressModalContent: VFC<{ closeModal?: () => void }> = ({ closeModal }) => {
  const [, rerender] = useState(0);

  useEffect(() => {
    return progressState.subscribe(() => rerender((n) => n + 1));
  }, []);

  const { operation, progress } = progressState;
  if (!operation) return null;

  const isInstalling = operation.type === "install";
  const isComplete = operation.status === "complete";
  const isError = operation.status === "error";
  const pct = (progress?.percentage ?? operation.progress) || 0;

  const statusText = isError
    ? `Error: ${operation.message}`
    : isComplete
      ? (isInstalling ? "Installed!" : "Removed!")
      : isInstalling ? "Installing..." : "Removing...";

  const statusClass = isError
    ? "cd-modal-status cd-modal-status-error"
    : isComplete
      ? "cd-modal-status cd-modal-status-done"
      : "cd-modal-status";

  return (
    <ModalRoot closeModal={closeModal}>
      <style>{getModalCSS()}</style>
      <div className="cd-modal-progress">
        <img src={mascotUrl} alt="" className="cd-modal-mascot" />
        <div className="cd-modal-title">
          {isInstalling ? "Installing game" : "Removing game"}
        </div>
        <div className="cd-modal-game">{operation.gameName}</div>

        {isComplete && <div className="cd-modal-check">✓</div>}

        {!isComplete && !isError && (
          <>
            <div className="cd-progress-bar">
              <div className="cd-progress-fill" style={{ width: `${pct}%` }} />
            </div>
            <div className="cd-progress-pct">{Math.round(pct)}%</div>
            {progress && (
              <div className="cd-progress-bytes">
                {formatBytes(progress.transferredBytes)} / {formatBytes(progress.totalBytes)}
              </div>
            )}
          </>
        )}

        <div className={statusClass}>{statusText}</div>
      </div>
    </ModalRoot>
  );
};

// ── Inline QAM panel (kept for in-panel view) ─────────────────────────────

interface ProgressPanelProps {
  operation: OperationEvent | null;
  uploadProgress: UploadProgress | null;
}

const ProgressPanel: VFC<ProgressPanelProps> = ({ operation, uploadProgress }) => {
  if (!operation) {
    return null;
  }

  const isInstalling = operation.type === "install";
  const isComplete = operation.status === "complete";
  const isError = operation.status === "error";

  const getStatusText = () => {
    if (isError) return `Error: ${operation.message}`;
    if (isComplete) return isInstalling ? "Installed!" : "Removed!";
    if (operation.status === "start") return isInstalling ? "Starting..." : "Removing...";
    return operation.message || "Processing...";
  };

  const getProgress = () => {
    if (uploadProgress && isInstalling && !isComplete && !isError) {
      return uploadProgress.percentage;
    }
    return operation.progress;
  };

  return (
    <div className="cd-section">
      <div className="cd-section-title">{isInstalling ? "Installing" : "Removing"}</div>
      <PanelSection>
      <PanelSectionRow>
        <Field label={operation.gameName} bottomSeparator="none">
          <span
            style={{
              color: isError ? colors.destructive : isComplete ? colors.primary : colors.foreground,
            }}
          >
            {getStatusText()}
          </span>
        </Field>
      </PanelSectionRow>

      {!isComplete && !isError && (
        <PanelSectionRow>
          <ProgressBarWithInfo
            nProgress={getProgress() / 100}
            sOperationText={
              uploadProgress
                ? `${formatBytes(uploadProgress.transferredBytes)} / ${formatBytes(uploadProgress.totalBytes)}`
                : `${Math.round(getProgress())}%`
            }
          />
        </PanelSectionRow>
      )}
      </PanelSection>
    </div>
  );
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export default ProgressPanel;
