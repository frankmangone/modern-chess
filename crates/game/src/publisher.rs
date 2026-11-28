//! Handing a game in, taking it back, and staff looking at it: the steps around the transactions of
//! `store::publish`.
//!
//! Before anything is written the rules are read by the engine, in the sandbox, and held to the
//! limits of a published game. A stranger's text is never run in this process.

use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde_json::Value;

use crate::store::{Author, Filed, Prepared, StoreError, UserGameRepo, Waiting};
use crate::{
    check_players, transition, DomainError, DraftError, Engine, EngineError, Event, GameStatus,
    Listing, Refusal, Slug, Staff, UserGame, UserGameId,
};

/// The longest reason staff may give for sending a game back, in characters.
pub const MAX_REASON_CHARS: usize = 1000;

#[derive(Clone)]
pub struct Publisher {
    games: UserGameRepo,
    engine: Engine,
    staff: Staff,
}

impl Publisher {
    pub fn new(games: UserGameRepo, engine: Engine, staff: Staff) -> Self {
        Self {
            games,
            engine,
            staff,
        }
    }

    /// An author hands in the draft they saw at `version`: it is published at once if one of their
    /// games has been approved before, and goes to review if not.
    pub async fn publish(
        &self,
        id: UserGameId,
        author: Author,
        version: i64,
        listing: Listing,
        now: DateTime<Utc>,
    ) -> Result<Filed, StoreError> {
        let listing = listing.checked()?;
        let game = self
            .games
            .get(id, author.id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if game.status != GameStatus::Draft {
            return Err(DraftError::NotADraft.into());
        }
        if game.version != version {
            return Err(DraftError::Stale {
                current: game.version,
            }
            .into());
        }
        let prepared = self.prepare(&game).await?;
        self.games
            .file(id, &author, version, &listing, &prepared, now)
            .await
    }

    /// An author takes a game back out of review.
    pub async fn withdraw(
        &self,
        id: UserGameId,
        author: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.games.withdraw(id, author, now).await
    }

    /// What staff have to look at: the games in review, the longest waiting first, or the one named.
    /// Only staff may ask, and their own games are not in it.
    pub async fn waiting(
        &self,
        reviewer: UserId,
        only: Option<UserGameId>,
    ) -> Result<Vec<Waiting>, StoreError> {
        if !self.staff.is_staff(reviewer) {
            return Err(Refusal::NotYours.into());
        }
        self.games.waiting(reviewer, only).await
    }

    /// Staff accept a game that is in review, and it goes live.
    pub async fn approve(
        &self,
        id: UserGameId,
        reviewer: UserId,
        now: DateTime<Utc>,
    ) -> Result<Slug, StoreError> {
        let (game, actor) = self.for_staff(id, reviewer).await?;
        // The move is asked of the machine before the engine is, so a game that is not in review
        // costs nothing.
        transition(game.status, Event::Approve, actor)?;
        let prepared = self.prepare(&game).await?;
        self.games
            .approve(id, reviewer, actor, &prepared, now)
            .await
    }

    /// Staff send a game that is in review back to its author, saying why.
    pub async fn reject(
        &self,
        id: UserGameId,
        reviewer: UserId,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let reason = reason.trim();
        if reason.is_empty() || reason.chars().count() > MAX_REASON_CHARS {
            return Err(DomainError::InvalidDraft(format!(
                "the reason must be 1 to {MAX_REASON_CHARS} characters"
            ))
            .into());
        }
        let (_, actor) = self.for_staff(id, reviewer).await?;
        self.games.reject(id, reviewer, actor, reason, now).await
    }

    /// A game and the part `reviewer` plays in it. A game that is not theirs, and that they are not
    /// staff for, does not exist for them.
    async fn for_staff(
        &self,
        id: UserGameId,
        reviewer: UserId,
    ) -> Result<(UserGame, crate::Actor), StoreError> {
        let game = self.games.get_any(id).await?.ok_or(StoreError::NotFound)?;
        let actor = self
            .staff
            .actor_for(reviewer, game.author)
            .ok_or(StoreError::NotFound)?;
        Ok((game, actor))
    }

    /// The draft's texts read as a game: rules the engine accepts and holds to the limits, and a
    /// look that is an object or nothing.
    async fn prepare(&self, game: &UserGame) -> Result<Prepared, StoreError> {
        let rules: Value = serde_json::from_str(&game.content.rules)
            .map_err(|err| DomainError::InvalidRules(format!("the rules are not JSON: {err}")))?;
        let summary = match self.engine.check(&rules).await {
            Ok(summary) => summary,
            Err(EngineError::Unusable(why)) => return Err(DomainError::InvalidRules(why).into()),
            Err(EngineError::Stopped(why)) => return Err(StoreError::Engine(why)),
        };
        check_players(summary.players.len())
            .map_err(|err| DomainError::InvalidRules(err.to_string()))?;

        let skin = match game.content.skin.trim() {
            "" => None,
            text => match serde_json::from_str::<Value>(text) {
                Ok(skin) if skin.is_object() => Some(skin),
                Ok(_) => {
                    return Err(
                        DomainError::InvalidDraft("the look must be an object".into()).into(),
                    )
                }
                Err(err) => {
                    return Err(
                        DomainError::InvalidDraft(format!("the look is not JSON: {err}")).into(),
                    )
                }
            },
        };
        Ok(Prepared {
            name: game.content.name.clone(),
            rules,
            skin,
        })
    }
}
