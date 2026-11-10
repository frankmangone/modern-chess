//! A throwaway sign-in provider for tests: RSA keys made on the spot, tokens signed the way a
//! real provider signs them, and a key source that counts how often it is asked. No key
//! material lives in the repository.
#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use modern_chess_social::provider::{KeyFuture, KeySource, ProviderError};
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::Serialize;
use serde_json::{json, Value};

pub const ISSUER: &str = "https://accounts.example.test";
pub const PROVIDER: &str = "testidp";

pub struct ProviderKeys {
    pub kid: String,
    private_pem: String,
    n: String,
    e: String,
}

impl ProviderKeys {
    fn generate(kid: &str) -> Self {
        let key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("generating a key");
        Self {
            kid: kid.to_string(),
            private_pem: key
                .to_pkcs8_pem(LineEnding::LF)
                .expect("encoding a fresh key")
                .to_string(),
            n: URL_SAFE_NO_PAD.encode(key.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(key.e().to_bytes_be()),
        }
    }

    /// The public half, in the shape a provider publishes it.
    pub fn jwk(&self) -> Value {
        json!({ "kty": "RSA", "alg": "RS256", "use": "sig", "kid": self.kid, "n": self.n, "e": self.e })
    }

    pub fn sign<T: Serialize>(&self, claims: &T) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        let key = EncodingKey::from_rsa_pem(self.private_pem.as_bytes()).expect("a valid key");
        encode(&header, claims, &key).expect("signing")
    }
}

/// Three key pairs, made once per test run (generating RSA keys is slow). Pair 0 is "the
/// provider's key", pair 1 is a second provider key for rotation, pair 2 is an attacker's.
pub fn keys(index: usize) -> &'static ProviderKeys {
    static KEYS: OnceLock<Vec<ProviderKeys>> = OnceLock::new();
    &KEYS.get_or_init(|| {
        let handles: Vec<_> = ["key-a", "key-b", "key-c"]
            .into_iter()
            .map(|kid| std::thread::spawn(move || ProviderKeys::generate(kid)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })[index]
}

pub fn jwks(of: &[&ProviderKeys]) -> JwkSet {
    let keys: Vec<Value> = of.iter().map(|k| k.jwk()).collect();
    serde_json::from_value(json!({ "keys": keys })).expect("a valid key set")
}

/// What a well-behaved token says right now.
pub fn claims(subject: &str) -> Value {
    let now = Utc::now().timestamp();
    json!({
        "iss": ISSUER,
        "sub": subject,
        "exp": now + 3600,
        "nbf": now - 10,
        "email": "player@example.test",
        "email_verified": true,
    })
}

/// A key source that serves whatever it is told to and counts the fetches.
pub struct CountingSource {
    keys: Mutex<JwkSet>,
    fetches: AtomicUsize,
    failing: AtomicBool,
}

impl CountingSource {
    pub fn new(keys: JwkSet) -> Self {
        Self {
            keys: Mutex::new(keys),
            fetches: AtomicUsize::new(0),
            failing: AtomicBool::new(false),
        }
    }

    /// While set, every fetch fails, as when the provider is down.
    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    pub fn publish(&self, keys: JwkSet) {
        *self.keys.lock().unwrap() = keys;
    }

    pub fn fetches(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }
}

impl KeySource for CountingSource {
    fn fetch(&self) -> KeyFuture<'_> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        let keys = self.keys.lock().unwrap().clone();
        let failing = self.failing.load(Ordering::SeqCst);
        Box::pin(async move {
            if failing {
                Err(ProviderError::Keys("the provider is down".to_string()))
            } else {
                Ok(keys)
            }
        })
    }
}
