//! Throwaway signing keys and tokens for tests. Dev-only: nothing ships this crate.
//!
//! Keys are generated fresh on every call and never written anywhere, so no key material
//! lives in the repository. It depends on the kernel only, so the game side can use it
//! without learning anything about the social module.

use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use ed25519_dalek::SigningKey;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use modern_chess_kernel::{Handle, TokenClaims, UserId};
use rand_core::OsRng;
use serde::Serialize;

/// One Ed25519 key pair, as PEM text.
pub struct TestKeys {
    pub private_pem: String,
    pub public_pem: String,
}

impl TestKeys {
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        Self {
            private_pem: signing
                .to_pkcs8_pem(LineEnding::LF)
                .expect("encoding a fresh key")
                .to_string(),
            public_pem: signing
                .verifying_key()
                .to_public_key_pem(LineEnding::LF)
                .expect("encoding a fresh key"),
        }
    }

    /// Signs any claims with this key, the way the real issuer would.
    pub fn sign<T: Serialize>(&self, claims: &T) -> String {
        let key = EncodingKey::from_ed_pem(self.private_pem.as_bytes()).expect("a valid key");
        encode(&Header::new(Algorithm::EdDSA), claims, &key).expect("signing")
    }

    /// A well-formed guest token for `user`, expiring at the Unix time `exp`.
    pub fn token_for(&self, user: UserId, handle: &str, exp: i64) -> String {
        self.sign(&claims(user, handle, exp))
    }
}

pub fn claims(user: UserId, handle: &str, exp: i64) -> TokenClaims {
    TokenClaims {
        sub: user,
        handle: Handle::parse(handle).expect("a valid handle"),
        exp,
    }
}

/// A token signed with HMAC and `secret`: what an attacker who treats the public key as a
/// shared secret would forge.
pub fn sign_with_hmac<T: Serialize>(secret: &[u8], claims: &T) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret),
    )
    .expect("signing")
}
