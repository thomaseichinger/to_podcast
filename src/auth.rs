use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

const SESSION_DURATION: Duration = Duration::from_secs(3600);

/// Verify a `Basic <base64(user:pass)>` Authorization header.
pub fn verify_basic_auth(header_value: &str, username: &str, password: &str) -> bool {
    let Some(encoded) = header_value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(bytes) = BASE64.decode(encoded.trim()) else {
        return false;
    };
    let Ok(creds) = std::str::from_utf8(&bytes) else {
        return false;
    };
    match creds.splitn(2, ':').collect::<Vec<_>>().as_slice() {
        [u, p] => *u == username && *p == password,
        _ => false,
    }
}

/// Thread-safe in-memory bearer token store with 1-hour expiry.
#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<String, Instant>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_session(&self) -> String {
        let token = Uuid::new_v4().to_string();
        self.inner
            .lock()
            .unwrap()
            .insert(token.clone(), Instant::now() + SESSION_DURATION);
        token
    }

    pub fn is_valid(&self, token: &str) -> bool {
        self.inner
            .lock()
            .unwrap()
            .get(token)
            .map(|&exp| Instant::now() < exp)
            .unwrap_or(false)
    }

    pub fn revoke(&self, token: &str) {
        self.inner.lock().unwrap().remove(token);
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_basic(u: &str, p: &str) -> String {
        format!("Basic {}", BASE64.encode(format!("{u}:{p}")))
    }

    #[test]
    fn test_verify_basic_auth_valid() {
        assert!(verify_basic_auth(&encode_basic("alice", "secret"), "alice", "secret"));
    }

    #[test]
    fn test_verify_basic_auth_wrong_password() {
        assert!(!verify_basic_auth(&encode_basic("alice", "wrong"), "alice", "secret"));
    }

    #[test]
    fn test_verify_basic_auth_wrong_user() {
        assert!(!verify_basic_auth(&encode_basic("bob", "secret"), "alice", "secret"));
    }

    #[test]
    fn test_verify_basic_auth_malformed_prefix() {
        assert!(!verify_basic_auth("Bearer sometoken", "alice", "secret"));
    }

    #[test]
    fn test_verify_basic_auth_invalid_base64() {
        assert!(!verify_basic_auth("Basic !!!notbase64!!!", "alice", "secret"));
    }

    #[test]
    fn test_verify_basic_auth_no_colon() {
        let encoded = BASE64.encode("alicesecret");
        assert!(!verify_basic_auth(&format!("Basic {}", encoded), "alice", "secret"));
    }

    #[test]
    fn test_session_create_and_validate() {
        let store = SessionStore::new();
        let token = store.create_session();
        assert!(store.is_valid(&token));
    }

    #[test]
    fn test_session_invalid_token() {
        let store = SessionStore::new();
        assert!(!store.is_valid("nonexistent-token"));
    }

    #[test]
    fn test_session_revoke() {
        let store = SessionStore::new();
        let token = store.create_session();
        store.revoke(&token);
        assert!(!store.is_valid(&token));
    }
}
