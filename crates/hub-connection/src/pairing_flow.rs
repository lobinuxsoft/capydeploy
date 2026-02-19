//! Hub-Agent pairing handshake logic.

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{
    AgentStatusResponse, HubConnectedRequest, PairConfirmRequest, PairFailedResponse,
    PairSuccessResponse, PairingRequiredResponse,
};

use crate::ws_client::{HandshakeResult, WsClient, WsError};

/// Performs the initial handshake after connecting to an Agent.
///
/// Returns [`HandshakeResult::Connected`] if the agent is already
/// authorised, or [`HandshakeResult::NeedsPairing`] if a PIN must
/// be confirmed first.
pub(crate) async fn perform_handshake(
    client: &WsClient,
    hub_request: &HubConnectedRequest,
) -> Result<HandshakeResult, WsError> {
    let resp = client
        .send_request(MessageType::HubConnected, Some(hub_request))
        .await?;

    // Check if pairing is required — return client alive.
    if resp.msg_type == MessageType::PairingRequired
        && let Ok(Some(pairing)) = resp.parse_payload::<PairingRequiredResponse>()
    {
        return Ok(HandshakeResult::NeedsPairing(pairing));
    }

    // Check for error response.
    if let Some(err) = &resp.error {
        return Err(WsError::AgentError {
            code: err.code,
            message: err.message.clone(),
        });
    }

    // Parse agent status.
    let status: AgentStatusResponse =
        resp.parse_payload::<AgentStatusResponse>()?
            .ok_or_else(|| WsError::AgentError {
                code: 500,
                message: "empty agent status".into(),
            })?;

    Ok(HandshakeResult::Connected(status))
}

/// Confirms a pairing code with the Agent.
///
/// Call this after receiving [`HandshakeResult::NeedsPairing`].
/// The connection must still be alive (same client instance).
pub(crate) async fn confirm_pairing(
    client: &WsClient,
    code: &str,
) -> Result<PairSuccessResponse, WsError> {
    let req = PairConfirmRequest {
        code: code.to_string(),
    };
    let resp = client
        .send_request(MessageType::PairConfirm, Some(&req))
        .await?;

    match resp.msg_type {
        MessageType::PairSuccess => {
            let success: PairSuccessResponse = resp
                .parse_payload::<PairSuccessResponse>()?
                .ok_or_else(|| WsError::PairingFailed("empty response".into()))?;
            Ok(success)
        }
        MessageType::PairFailed => {
            let failed = resp.parse_payload::<PairFailedResponse>()?;
            let reason = failed.map(|f| f.reason).unwrap_or_default();
            Err(WsError::PairingFailed(reason))
        }
        _ => Err(WsError::PairingFailed("unexpected response".into())),
    }
}
