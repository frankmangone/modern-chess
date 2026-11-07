use serde::{Deserialize, Serialize};

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
