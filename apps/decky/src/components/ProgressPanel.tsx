/**
 * ProgressPanel - Shows current transfer progress.
 */

import { PanelSection, PanelSectionRow, Field, ProgressBarWithInfo } from "@decky/ui";
import { VFC } from "react";
import type { OperationEvent, UploadProgress } from "../types";
import { colors } from "../styles/theme";

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
    if (isComplete) return isInstalling ? "Instalado!" : "Eliminado!";
    if (operation.status === "start") return isInstalling ? "Iniciando..." : "Eliminando...";
    return operation.message || "Procesando...";
  };

  const getProgress = () => {
    if (uploadProgress && isInstalling && operation.status === "progress") {
      return uploadProgress.percentage;
    }
    return operation.progress;
  };

  return (
    <PanelSection title={isInstalling ? "Instalando" : "Eliminando"}>
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
