/// Generates a stable agent ID from the name.
///
/// Produces an 8-char hex string derived from SHA-256(`{name}-{platform}-agent`).
pub(crate) fn generate_agent_id(name: &str) -> String {
    use std::fmt::Write;
    let platform = std::env::consts::OS;
    let data = format!("{name}-{platform}-agent");
    let digest = <sha2::Sha256 as sha2::Digest>::digest(data.as_bytes());
    let mut hex = String::with_capacity(8);
    for byte in &digest[..4] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}
