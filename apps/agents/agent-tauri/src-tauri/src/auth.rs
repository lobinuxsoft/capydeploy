//! Pairing code generation, validation, and token management.
//!
//! Port of Go `apps/agents/desktop/auth/auth.go`.

use std::time::{Duration, Instant};

use base64::Engine;

/// Code length (6 digits).
const CODE_LENGTH: usize = 6;
/// Code validity window.
const CODE_EXPIRY: Duration = Duration::from_secs(60);
/// Token length in random bytes.
const TOKEN_LENGTH: usize = 32;
/// Max failed attempts before rate limiting.
const MAX_FAILED_ATTEMPTS: u32 = 3;
/// Rate limit duration after max failed attempts.
const RATE_LIMIT_DURATION: Duration = Duration::from_secs(300);

/// Active pairing session.
#[derive(Debug, Clone)]
pub struct PairingSession {
    pub code: String,
    pub hub_id: String,
    pub hub_name: String,
    pub hub_platform: String,
    pub expires_at: Instant,
}

/// Manages pairing codes and token validation.
pub struct AuthManager {
    pending: Option<PairingSession>,
    failed_attempts: u32,
    rate_limit_until: Option<Instant>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            pending: None,
            failed_attempts: 0,
            rate_limit_until: None,
        }
    }

    /// Generates a 6-digit pairing code for the given Hub.
    pub fn generate_code(
        &mut self,
        hub_id: &str,
        hub_name: &str,
        hub_platform: &str,
    ) -> Result<String, AuthError> {
        // Check rate limiting
        if let Some(until) = self.rate_limit_until {
            if Instant::now() < until {
                return Err(AuthError::RateLimited);
            }
            self.rate_limit_until = None;
        }

        let code = generate_numeric_code(CODE_LENGTH)?;

        self.pending = Some(PairingSession {
            code: code.clone(),
            hub_id: hub_id.to_string(),
            hub_name: hub_name.to_string(),
            hub_platform: hub_platform.to_string(),
            expires_at: Instant::now() + CODE_EXPIRY,
        });

        Ok(code)
    }

    /// Validates a pairing code. Returns a token on success.
    pub fn validate_code(
        &mut self,
        hub_id: &str,
        _hub_name: &str,
        code: &str,
    ) -> Result<String, AuthError> {
        // Check rate limiting
        if let Some(until) = self.rate_limit_until {
            if Instant::now() < until {
                return Err(AuthError::RateLimited);
            }
            self.rate_limit_until = None;
        }

        let session = self.pending.as_ref().ok_or(AuthError::NoPendingPairing)?;

        // Check expiration
        if Instant::now() > session.expires_at {
            self.pending = None;
            return Err(AuthError::CodeExpired);
        }

        // Check Hub ID matches
        if session.hub_id != hub_id {
            self.record_failure();
            return Err(AuthError::CodeInvalid);
        }

        // Validate code
        if session.code != code {
            self.record_failure();
            return Err(AuthError::CodeInvalid);
        }

        // Generate token
        let token = generate_token(TOKEN_LENGTH)?;

        // Clear state
        self.pending = None;
        self.failed_attempts = 0;

        Ok(token)
    }

    /// Checks if a Hub's token matches one in the authorized list.
    pub fn validate_token(authorized_hubs: &[crate::config::AuthorizedHub], hub_id: &str, token: &str) -> bool {
        authorized_hubs
            .iter()
            .any(|h| h.id == hub_id && h.token == token)
    }

    /// Returns the pending pairing session (if not expired).
    pub fn pending_pairing(&self) -> Option<&PairingSession> {
        self.pending.as_ref().filter(|s| Instant::now() < s.expires_at)
    }

    /// Cancels any pending pairing session.
    #[allow(dead_code)]
    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }

    fn record_failure(&mut self) {
        self.failed_attempts += 1;
        if self.failed_attempts >= MAX_FAILED_ATTEMPTS {
            self.rate_limit_until = Some(Instant::now() + RATE_LIMIT_DURATION);
            self.failed_attempts = 0;
        }
    }
}

/// Auth errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("pairing code expired")]
    CodeExpired,
    #[error("invalid pairing code")]
    CodeInvalid,
    #[error("too many failed attempts, try again later")]
    RateLimited,
    #[error("no pending pairing")]
    NoPendingPairing,
}

/// Generates a random n-digit numeric code.
fn generate_numeric_code(length: usize) -> Result<String, AuthError> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let code: String = (0..length)
        .map(|_| {
            let digit: u8 = rng.gen_range(0..10);
            char::from(b'0' + digit)
        })
        .collect();
    Ok(code)
}

/// Generates a random base64url-encoded token.
fn generate_token(length: usize) -> Result<String, AuthError> {
    use rand::RngCore;
    let mut bytes = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut bytes);
    Ok(base64::engine::general_purpose::URL_SAFE.encode(&bytes))
}
