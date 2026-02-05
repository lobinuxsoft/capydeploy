/**
 * NameEditModal - Modal popup for editing the agent name.
 */

import { ModalRoot, TextField } from "@decky/ui";
import { call } from "@decky/api";
import { VFC, useState } from "react";
import { getModalCSS } from "../styles/theme";

import mascotUrl from "../../assets/mascot.gif";

interface NameEditModalProps {
  closeModal?: () => void;
  currentName: string;
  onSaved: () => void;
}

const NameEditModal: VFC<NameEditModalProps> = ({ closeModal, currentName, onSaved }) => {
  const [name, setName] = useState(currentName);
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      await call<[string], void>("set_agent_name", name.trim());
      onSaved();
      closeModal?.();
    } catch (e) {
      console.error("Failed to save name:", e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <ModalRoot closeModal={closeModal}>
      <style>{getModalCSS()}</style>
      <div className="cd-modal-progress">
        <img src={mascotUrl} alt="" className="cd-modal-mascot" />
        <div className="cd-modal-title">Rename Agent</div>
        <div className="cd-modal-subtitle">
          Choose a name for this device
        </div>
        <div className="cd-modal-input-wrap">
          <TextField
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={saving}
          />
        </div>
        <div className="cd-modal-actions">
          <button
            className="cd-modal-btn cd-modal-btn-primary"
            onClick={handleSave}
            disabled={saving || !name.trim()}
          >
            {saving ? "Saving..." : "Save"}
          </button>
          <button
            className="cd-modal-btn cd-modal-btn-secondary"
            onClick={closeModal}
            disabled={saving}
          >
            Cancel
          </button>
        </div>
      </div>
    </ModalRoot>
  );
};

export default NameEditModal;
