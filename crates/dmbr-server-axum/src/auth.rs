//! Minimal stateless session auth: an HMAC-SHA256-signed cookie carrying the
//! admin username. No server-side session store — the signature proves the
//! cookie was issued by us. Sufficient for the admin UI's scope.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The cookie name holding the signed session.
const COOKIE_NAME: &str = "dmbr_admin";

/// Holds the signing key derived from the configured secret.
#[derive(Clone)]
pub struct SessionKey {
    secret: Vec<u8>,
}

impl SessionKey {
    /// Creates a key from raw secret bytes.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    /// Computes the hex HMAC of `msg`.
    fn sign(&self, msg: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(msg.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Verifies a `username|sig` token, returning the username on a valid sig.
    fn verify(&self, token: &str) -> Option<String> {
        let (user, sig) = token.rsplit_once('|')?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(user.as_bytes());
        let bytes = hex::decode(sig).ok()?;
        mac.verify_slice(&bytes).ok()?;
        Some(user.to_string())
    }
}

/// Builds a `Set-Cookie` header value establishing the session for `username`.
pub fn make_cookie(key: &SessionKey, username: &str) -> String {
    let token = format!("{username}|{}", key.sign(username));
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400")
}

/// Builds a `Set-Cookie` header value that clears the session.
pub fn clear_cookie() -> String {
    format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

/// Extracts and verifies the session from a `Cookie` header string, returning
/// the authenticated username if present and valid.
pub fn session_user(key: &SessionKey, cookie_header: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{COOKIE_NAME}=")) {
            return key.verify(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie_value(set_cookie: &str) -> String {
        set_cookie
            .strip_prefix("dmbr_admin=")
            .and_then(|s| s.split(';').next())
            .unwrap()
            .to_string()
    }

    #[test]
    fn round_trips_a_valid_session() {
        let key = SessionKey::new(b"test-secret");
        let value = cookie_value(&make_cookie(&key, "admin"));
        let header = format!("dmbr_admin={value}");
        assert_eq!(session_user(&key, &header).as_deref(), Some("admin"));
    }

    #[test]
    fn rejects_tampered_session() {
        let key = SessionKey::new(b"test-secret");
        let header = "dmbr_admin=admin|deadbeef".to_string();
        assert_eq!(session_user(&key, &header), None);
    }

    #[test]
    fn rejects_wrong_secret() {
        let issuer = SessionKey::new(b"secret-a");
        let value = cookie_value(&make_cookie(&issuer, "admin"));
        let attacker = SessionKey::new(b"secret-b");
        let header = format!("dmbr_admin={value}");
        assert_eq!(session_user(&attacker, &header), None);
    }
}
