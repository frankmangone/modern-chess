//! Which game pages have a discussion.
//!
//! The social service has no games. A thread belongs to a game page by its slug, and whether that
//! page exists and is open is the game service's to say, so it is asked, through [`GamePages`]. The
//! real one, [`HttpGamePages`], reads the game service's public page. Tests supply their own.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use thiserror::Error;

/// How long an answer is kept. This is also the lag between a game's page being taken off the hub
/// and its discussion going with it.
pub const ANSWER_TTL: Duration = Duration::from_secs(60);

/// The most answers kept at once. A slug is only asked about after it looks like one, but anyone
/// can make up slugs, so the memory is bounded: when full it starts over.
const MAX_ANSWERS: usize = 1000;

#[derive(Debug, Error)]
pub enum PagesError {
    /// The game service could not say. Treated as "not now", never as "closed".
    #[error("the game service could not be asked: {0}")]
    Unavailable(String),
}

pub type OpenFuture<'a> = Pin<Box<dyn Future<Output = Result<bool, PagesError>> + Send + 'a>>;

/// Whether a game page exists and is open to the public.
pub trait GamePages: Send + Sync {
    fn is_open(&self, slug: &str) -> OpenFuture<'_>;
}

/// A game's address as the game service spells it: lowercase letters, digits and `-`, 1 to 64
/// characters. Checked here too, so nothing else is ever put in the address of a request.
pub fn looks_like_a_slug(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Asks the game service's public page, `GET {base}/catalog/{slug}`: 200 is open, 404 is not.
/// Definite answers are kept for [`ANSWER_TTL`]; a failure is not kept.
pub struct HttpGamePages {
    client: reqwest::Client,
    base: String,
    answers: Mutex<HashMap<String, (bool, Instant)>>,
}

impl HttpGamePages {
    /// `base` is the game service's address up to and including `/api/game`.
    pub fn new(base: &str) -> Result<Self, PagesError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|err| PagesError::Unavailable(err.to_string()))?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            answers: Mutex::new(HashMap::new()),
        })
    }

    fn remembered(&self, slug: &str) -> Option<bool> {
        let answers = self.answers.lock().unwrap_or_else(|e| e.into_inner());
        answers
            .get(slug)
            .filter(|(_, at)| at.elapsed() < ANSWER_TTL)
            .map(|(open, _)| *open)
    }

    fn remember(&self, slug: &str, open: bool) {
        let mut answers = self.answers.lock().unwrap_or_else(|e| e.into_inner());
        if answers.len() >= MAX_ANSWERS {
            answers.clear();
        }
        answers.insert(slug.to_string(), (open, Instant::now()));
    }
}

impl GamePages for HttpGamePages {
    fn is_open(&self, slug: &str) -> OpenFuture<'_> {
        let slug = slug.to_string();
        Box::pin(async move {
            if !looks_like_a_slug(&slug) {
                return Ok(false);
            }
            if let Some(open) = self.remembered(&slug) {
                return Ok(open);
            }
            let response = self
                .client
                .get(format!("{}/catalog/{slug}", self.base))
                .send()
                .await
                .map_err(|err| PagesError::Unavailable(err.to_string()))?;
            let open = match response.status() {
                StatusCode::OK => true,
                StatusCode::NOT_FOUND => false,
                other => return Err(PagesError::Unavailable(format!("status {other}"))),
            };
            self.remember(&slug, open);
            Ok(open)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_a_slug;

    #[test]
    fn only_slug_shaped_text_is_asked_about() {
        assert!(looks_like_a_slug("pong-hau-kitchen-2"));
        assert!(!looks_like_a_slug(""));
        assert!(!looks_like_a_slug("Upper"));
        assert!(!looks_like_a_slug("../etc"));
        assert!(!looks_like_a_slug("a/b"));
        assert!(!looks_like_a_slug(&"a".repeat(65)));
    }
}
