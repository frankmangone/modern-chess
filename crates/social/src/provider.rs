//! Checking a token from an outside sign-in provider (Clerk today).
//!
//! Nothing here is specific to one vendor. Any provider that signs RS256 tokens and publishes
//! its public keys as a JWKS works, so changing provider means changing settings, not code.
//! The verifier needs no secret: it only reads public keys. A provider's token is seen once,
//! when it is exchanged for one of our own, and never travels any further.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tracing::warn;

/// Provider clocks drift a little; a token is honoured this long around its time limits.
const LEEWAY_SECONDS: i64 = 5;
/// Keys are refetched for an unknown key id at most this often, so a stream of made-up key
/// ids cannot turn this server into a way to hammer the provider.
const REFETCH_COOLDOWN: Duration = Duration::from_secs(60);
/// Keys older than this are fetched again, so a key the provider has retired or revoked stops
/// being trusted within the hour instead of when the server next restarts.
const KEYS_MAX_AGE: Duration = Duration::from_secs(60 * 60);
const MAX_SUBJECT_LEN: usize = 255;
const MAX_EMAIL_LEN: usize = 254;
const MAX_KEYS_BYTES: usize = 1024 * 1024;

/// Which provider to trust, from settings.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// The label stored with every identity, such as `clerk`. A new provider gets a new label.
    pub name: String,
    /// The `iss` every token must carry.
    pub issuer: String,
    /// Where the provider publishes its public keys.
    pub jwks_url: String,
    /// Origins allowed to have asked for the token (the `azp` claim). Empty means any. An entry
    /// may hold one `*` standing for one host label, like `https://*.example.com`. Do not
    /// list a domain anyone can get a subdomain of: that allows everyone.
    pub authorized_parties: Vec<String>,
}

/// Who a provider says a token belongs to. This is all the rest of the product learns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentity {
    pub provider: String,
    pub subject: String,
    /// Present only when the provider vouches for it.
    pub email: Option<String>,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    /// The token is not acceptable. Deliberately says nothing about which check failed.
    #[error("the credential is not valid")]
    Invalid,
    /// The provider's keys could not be loaded. The caller's token may be fine.
    #[error("could not load the provider's keys: {0}")]
    Keys(String),
}

pub type KeyFuture<'a> = Pin<Box<dyn Future<Output = Result<JwkSet, ProviderError>> + Send + 'a>>;

/// Where public keys come from. The real one is [`HttpKeySource`]; tests supply their own.
pub trait KeySource: Send + Sync {
    fn fetch(&self) -> KeyFuture<'_>;
}

/// Downloads the key set over HTTPS.
pub struct HttpKeySource {
    client: reqwest::Client,
    url: String,
}

impl HttpKeySource {
    pub fn new(url: &str) -> Result<Self, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|err| ProviderError::Keys(err.to_string()))?;
        Ok(Self {
            client,
            url: url.to_string(),
        })
    }
}

impl KeySource for HttpKeySource {
    fn fetch(&self) -> KeyFuture<'_> {
        Box::pin(async move {
            let keys_error = |err: &dyn std::fmt::Display| ProviderError::Keys(err.to_string());
            let response = self
                .client
                .get(&self.url)
                .send()
                .await
                .map_err(|e| keys_error(&e))?;
            if !response.status().is_success() {
                return Err(ProviderError::Keys(format!("status {}", response.status())));
            }
            let body = response.bytes().await.map_err(|e| keys_error(&e))?;
            if body.len() > MAX_KEYS_BYTES {
                return Err(ProviderError::Keys("the key set is too large".to_string()));
            }
            serde_json::from_slice(&body).map_err(|e| keys_error(&e))
        })
    }
}

struct KeyCache {
    keys: JwkSet,
    /// When the keys were last asked for, whether or not that worked.
    fetched_at: Option<Instant>,
    /// When the keys last arrived.
    loaded_at: Option<Instant>,
}

/// Checks provider tokens against the provider's published keys.
pub struct ProviderVerifier {
    name: String,
    issuer: String,
    authorized_parties: Vec<String>,
    source: Arc<dyn KeySource>,
    cooldown: Duration,
    max_age: Duration,
    cache: RwLock<KeyCache>,
    /// One fetch at a time. Held during the download, while the cache itself is not, so tokens
    /// signed with keys we already have never wait for the provider.
    fetching: Mutex<()>,
}

#[derive(Deserialize)]
struct ProviderClaims {
    iss: String,
    sub: String,
    exp: i64,
    nbf: Option<i64>,
    azp: Option<String>,
    email: Option<String>,
    email_verified: Option<bool>,
}

impl ProviderVerifier {
    pub fn new(config: ProviderConfig, source: Arc<dyn KeySource>) -> Self {
        Self {
            name: config.name,
            issuer: config.issuer,
            authorized_parties: config.authorized_parties,
            source,
            cooldown: REFETCH_COOLDOWN,
            max_age: KEYS_MAX_AGE,
            cache: RwLock::new(KeyCache {
                keys: JwkSet { keys: Vec::new() },
                fetched_at: None,
                loaded_at: None,
            }),
            fetching: Mutex::new(()),
        }
    }

    /// A verifier that downloads the provider's keys from `config.jwks_url`.
    pub fn connect(config: ProviderConfig) -> Result<Self, ProviderError> {
        let source = HttpKeySource::new(&config.jwks_url)?;
        Ok(Self::new(config, Arc::new(source)))
    }

    /// Changes how soon keys may be asked for again.
    pub fn with_refetch_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Changes how long downloaded keys are trusted before they are fetched again.
    pub fn with_keys_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// The identity a genuine, current token names.
    ///
    /// Only RS256 is accepted, so a token signed another way (HMAC keyed with a public value,
    /// or no signature at all) is refused. Time limits are checked against `now`, not the
    /// library's clock, so they can be tested.
    pub async fn verify(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<ExternalIdentity, ProviderError> {
        let header = decode_header(token).map_err(|_| ProviderError::Invalid)?;
        if header.alg != Algorithm::RS256 {
            return Err(ProviderError::Invalid);
        }
        let kid = header.kid.ok_or(ProviderError::Invalid)?;
        let key = self.decoding_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        validation.required_spec_claims.clear();
        let claims = decode::<ProviderClaims>(token, &key, &validation)
            .map_err(|_| ProviderError::Invalid)?
            .claims;

        self.check(&claims, now)?;
        Ok(ExternalIdentity {
            provider: self.name.clone(),
            subject: claims.sub,
            email: claims.email.filter(|email| {
                claims.email_verified == Some(true) && email.len() <= MAX_EMAIL_LEN
            }),
        })
    }

    fn check(&self, claims: &ProviderClaims, now: DateTime<Utc>) -> Result<(), ProviderError> {
        let now = now.timestamp();
        let acceptable = claims.iss == self.issuer
            && claims.exp + LEEWAY_SECONDS > now
            && claims.nbf.is_none_or(|nbf| nbf - LEEWAY_SECONDS <= now)
            && !claims.sub.is_empty()
            && claims.sub.len() <= MAX_SUBJECT_LEN
            && self.origin_allowed(claims.azp.as_deref());
        if acceptable {
            Ok(())
        } else {
            Err(ProviderError::Invalid)
        }
    }

    /// With a list configured, a token that names an origin must name one on it. A token that
    /// names none passes: the provider's documentation says to skip the check then.
    fn origin_allowed(&self, azp: Option<&str>) -> bool {
        match azp {
            Some(origin) if !self.authorized_parties.is_empty() => self
                .authorized_parties
                .iter()
                .any(|pattern| origin_matches(pattern, origin)),
            _ => true,
        }
    }

    /// The key for `kid`, from the cache while it is fresh, otherwise from the provider.
    ///
    /// Asking the provider is limited by the cooldown. When it fails, or may not be asked yet,
    /// an older copy of a key is still used: a provider outage does not sign everyone out.
    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, ProviderError> {
        if let Some(key) = self.fresh_key(kid).await {
            return Ok(key);
        }

        let _one_at_a_time = self.fetching.lock().await;
        // Another request may have refreshed the keys while this one waited its turn.
        if let Some(key) = self.fresh_key(kid).await {
            return Ok(key);
        }

        let may_fetch = match self.cache.read().await.fetched_at {
            Some(at) => at.elapsed() >= self.cooldown,
            None => true,
        };
        if !may_fetch {
            return self.remembered_key(kid, ProviderError::Invalid).await;
        }

        // Noted before fetching, so a provider that is down is not retried on every request.
        self.cache.write().await.fetched_at = Some(Instant::now());
        match self.source.fetch().await {
            Ok(keys) => {
                let mut cache = self.cache.write().await;
                cache.keys = keys;
                cache.loaded_at = Some(Instant::now());
            }
            Err(err) => {
                warn!("could not refresh the provider's keys: {err}");
                return self.remembered_key(kid, err).await;
            }
        }
        self.remembered_key(kid, ProviderError::Invalid).await
    }

    async fn fresh_key(&self, kid: &str) -> Option<DecodingKey> {
        let cache = self.cache.read().await;
        let fresh = cache
            .loaded_at
            .is_some_and(|at| at.elapsed() < self.max_age);
        if fresh {
            find_key(&cache.keys, kid)
        } else {
            None
        }
    }

    /// The cached key for `kid`, however old. With nothing cached at all the keys have never
    /// loaded, which is the server's problem and not the caller's, so that is reported as a
    /// failure to load keys rather than as a bad token.
    async fn remembered_key(
        &self,
        kid: &str,
        otherwise: ProviderError,
    ) -> Result<DecodingKey, ProviderError> {
        let cache = self.cache.read().await;
        if let Some(key) = find_key(&cache.keys, kid) {
            return Ok(key);
        }
        if cache.loaded_at.is_none() {
            return Err(match otherwise {
                ProviderError::Keys(reason) => ProviderError::Keys(reason),
                ProviderError::Invalid => {
                    ProviderError::Keys("the provider's keys have not loaded yet".to_string())
                }
            });
        }
        Err(otherwise)
    }
}

fn find_key(keys: &JwkSet, kid: &str) -> Option<DecodingKey> {
    keys.find(kid)
        .and_then(|jwk| DecodingKey::from_jwk(jwk).ok())
}

/// Exact match, or a pattern with one `*` standing for exactly one host label (letters, digits
/// and `-`), so `https://*.example.com` matches `https://a.example.com` but not
/// `https://a.b.example.com`, and cannot be satisfied by a path or a query string.
fn origin_matches(pattern: &str, origin: &str) -> bool {
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        return pattern == origin;
    };
    let Some(middle) = origin
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
    else {
        return false;
    };
    !middle.is_empty()
        && middle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}
