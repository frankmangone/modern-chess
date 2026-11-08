use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Handle, UserId};

/// What a signed guest token says. Issued by social-api, verified offline by game-api.
///
/// `exp` is a Unix timestamp in seconds, the name the JWT standard uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: UserId,
    pub handle: Handle,
    pub exp: i64,
}

/// Why a token or a key was refused. Deliberately coarse: callers answer 401 for all of
/// the token cases and never say which check failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TokenError {
    #[error("not a valid Ed25519 key")]
    InvalidKey,
    #[error("the token is not valid")]
    Invalid,
    #[error("the token has expired")]
    Expired,
}

/// Checks tokens with the public half of social-api's signing key. It holds no secret and
/// cannot sign, so the game side can verify a user without ever being able to forge one.
#[derive(Clone)]
pub struct TokenVerifier {
    key: DecodingKey,
}

impl std::fmt::Debug for TokenVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenVerifier(..)")
    }
}

impl TokenVerifier {
    /// Reads an Ed25519 public key in PEM form (`-----BEGIN PUBLIC KEY-----`).
    pub fn from_public_pem(pem: &str) -> Result<Self, TokenError> {
        DecodingKey::from_ed_pem(pem.as_bytes())
            .map(|key| Self { key })
            .map_err(|_| TokenError::InvalidKey)
    }

    /// Returns the claims of a genuine token that has not expired at `now`.
    ///
    /// Only EdDSA is accepted, so a token signed with another algorithm (for example HMAC
    /// keyed with the public key) is refused. Expiry is checked here against `now`, not
    /// against the library's clock, so it can be tested.
    pub fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<TokenClaims, TokenError> {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.validate_exp = false;
        validation.required_spec_claims.clear();

        let claims = decode::<TokenClaims>(token, &self.key, &validation)
            .map_err(|_| TokenError::Invalid)?
            .claims;

        if claims.exp <= now.timestamp() {
            return Err(TokenError::Expired);
        }
        Ok(claims)
    }
}
