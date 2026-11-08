use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePublicKey};
use ed25519_dalek::SigningKey;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use modern_chess_kernel::{TokenClaims, TokenError};

use crate::User;

/// How long a guest token stays valid. Tokens cannot be revoked, so this is the limit of
/// how long a leaked one is useful.
const TOKEN_LIFETIME_DAYS: i64 = 7;

/// Signs guest tokens with social-api's private key. This type, and the key it holds, exist
/// only in this module: the game side links the kernel's verifier and can never sign.
pub struct TokenIssuer {
    key: EncodingKey,
    public_pem: String,
}

impl TokenIssuer {
    /// Reads an Ed25519 private key in PKCS#8 PEM form.
    pub fn from_private_pem(pem: &str) -> Result<Self, TokenError> {
        let signing = SigningKey::from_pkcs8_pem(pem).map_err(|_| TokenError::InvalidKey)?;
        let public_pem = signing
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .map_err(|_| TokenError::InvalidKey)?;
        let key = EncodingKey::from_ed_pem(pem.as_bytes()).map_err(|_| TokenError::InvalidKey)?;
        Ok(Self { key, public_pem })
    }

    /// The matching public key, which is what the game side is configured with.
    pub fn public_pem(&self) -> &str {
        &self.public_pem
    }

    /// A token naming `user`, valid for seven days from `now`.
    pub fn issue(&self, user: &User, now: DateTime<Utc>) -> Result<String, TokenError> {
        let claims = TokenClaims {
            sub: user.id,
            handle: user.handle.clone(),
            exp: (now + Duration::days(TOKEN_LIFETIME_DAYS)).timestamp(),
        };
        encode(&Header::new(Algorithm::EdDSA), &claims, &self.key).map_err(|_| TokenError::Invalid)
    }
}
