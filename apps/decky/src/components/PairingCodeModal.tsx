/**
 * PairingCodeModal - Styled modal showing the pairing code for Hub linking.
 */

import { ModalRoot } from "@decky/ui";
import { VFC } from "react";
import { getModalCSS } from "../styles/theme";

import mascotUrl from "../../assets/mascot.gif";

interface PairingCodeModalProps {
  closeModal?: () => void;
  code: string;
}

const PairingCodeModal: VFC<PairingCodeModalProps> = ({ closeModal, code }) => {
  return (
    <ModalRoot closeModal={closeModal}>
      <style>{getModalCSS()}</style>
      <div className="cd-modal-progress">
        <img src={mascotUrl} alt="" className="cd-modal-mascot" />
        <div className="cd-modal-title">Pairing Code</div>
        <div className="cd-modal-subtitle">
          Enter this code in the Hub to link this device
        </div>
        <div className="cd-modal-code">{code}</div>
        <div className="cd-modal-actions">
          <button
            className="cd-modal-btn cd-modal-btn-primary"
            onClick={closeModal}
          >
            Got it
          </button>
        </div>
      </div>
    </ModalRoot>
  );
};

export default PairingCodeModal;
