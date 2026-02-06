/**
 * ConfirmActionModal - Branded confirmation dialog with glass effect.
 * Replaces Steam's generic ConfirmModal with CapyDeploy styling.
 */

import { ModalRoot } from "@decky/ui";
import { VFC } from "react";
import { getModalCSS } from "../styles/theme";

import mascotUrl from "../../assets/mascot.gif";

interface ConfirmActionModalProps {
  closeModal?: () => void;
  title: string;
  description: string;
  confirmText?: string;
  cancelText?: string;
  destructive?: boolean;
  onConfirm: () => void;
}

const ConfirmActionModal: VFC<ConfirmActionModalProps> = ({
  closeModal,
  title,
  description,
  confirmText = "Confirm",
  cancelText = "Cancel",
  destructive = false,
  onConfirm,
}) => {
  const handleConfirm = () => {
    onConfirm();
    closeModal?.();
  };

  return (
    <ModalRoot closeModal={closeModal}>
      <style>{getModalCSS()}</style>
      <div className="cd-modal-progress">
        <img src={mascotUrl} alt="" className="cd-modal-mascot" />
        <div className="cd-modal-title">{title}</div>
        <div className="cd-modal-subtitle">{description}</div>
        <div className="cd-modal-actions">
          <button
            className={`cd-modal-btn ${destructive ? "cd-modal-btn-danger" : "cd-modal-btn-primary"}`}
            onClick={handleConfirm}
          >
            {confirmText}
          </button>
          <button
            className="cd-modal-btn cd-modal-btn-secondary"
            onClick={closeModal}
          >
            {cancelText}
          </button>
        </div>
      </div>
    </ModalRoot>
  );
};

export default ConfirmActionModal;
