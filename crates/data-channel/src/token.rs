//! Cryptographically secure token generation and validation.

use rand::Rng;

/// Token length in bytes (produces 32 hex characters).
const TOKEN_BYTES: usize = 16;

/// Generates a CSPRNG token as a 32-character lowercase hex string.
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// Validates a received token against the expected value.
///
/// Uses constant-time comparison to prevent timing attacks.
pub fn validate_token(received: &str, expected: &str) -> bool {
    if received.len() != expected.len() {
        return false;
    }
    // Constant-time comparison.
    let mut diff = 0u8;
    for (a, b) in received.bytes().zip(expected.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 32);
    }

    #[test]
    fn generated_token_is_hex() {
        let token = generate_token();
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn tokens_are_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
    }

    #[test]
    fn validate_matching_tokens() {
        let token = generate_token();
        assert!(validate_token(&token, &token));
    }

    #[test]
    fn validate_mismatched_tokens() {
        let a = generate_token();
        let b = generate_token();
        assert!(!validate_token(&a, &b));
    }

    #[test]
    fn validate_different_lengths() {
        assert!(!validate_token("short", "this_is_longer"));
    }
}
