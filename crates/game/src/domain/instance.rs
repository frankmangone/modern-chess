use std::collections::HashSet;

use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use serde_json::Value;

use super::{
    DomainError, GameSpec, InstanceId, InstanceStatus, LobbyError, Outcome, Player, SeatName, Slug,
    SpecId,
};

/// One lobby or match of a game.
///
/// Fields are private because they must agree with each other: a lobby has empty seats, an
/// active game has none, a finished game has an outcome. Changes go through methods, and
/// rows read back from a database go through [`GameInstance::restore`], which checks them.
#[derive(Debug, Clone, PartialEq)]
pub struct GameInstance {
    id: InstanceId,
    spec_id: SpecId,
    slug: Slug,
    spec_version: u32,
    status: InstanceStatus,
    seats: Vec<SeatName>,
    players: Vec<Player>,
    state: Value,
    revision: u64,
    outcome: Option<Outcome>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

/// Every field of an instance, for saving it and for restoring it.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceParts {
    pub id: InstanceId,
    pub spec_id: SpecId,
    pub slug: Slug,
    pub spec_version: u32,
    pub status: InstanceStatus,
    pub seats: Vec<SeatName>,
    pub players: Vec<Player>,
    pub state: Value,
    pub revision: u64,
    pub outcome: Option<Outcome>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl GameInstance {
    /// An empty lobby at the opening position of `spec`.
    pub fn new_lobby(spec: &GameSpec, now: DateTime<Utc>) -> Result<Self, DomainError> {
        let seats = spec.seats()?;
        if seats.is_empty() {
            return Err(DomainError::InvalidInstance(
                "a game needs at least one seat".to_string(),
            ));
        }

        Ok(Self {
            id: InstanceId::new(),
            spec_id: spec.id,
            slug: spec.slug.clone(),
            spec_version: spec.version,
            status: InstanceStatus::Lobby,
            seats,
            players: Vec::new(),
            state: spec.initial_state()?,
            revision: 0,
            outcome: None,
            created_at: now,
            started_at: None,
            finished_at: None,
        })
    }

    /// Rebuilds an instance from stored parts, refusing parts that contradict each other.
    pub fn restore(parts: InstanceParts) -> Result<Self, DomainError> {
        check_parts(&parts)?;
        Ok(Self {
            id: parts.id,
            spec_id: parts.spec_id,
            slug: parts.slug,
            spec_version: parts.spec_version,
            status: parts.status,
            seats: parts.seats,
            players: parts.players,
            state: parts.state,
            revision: parts.revision,
            outcome: parts.outcome,
            created_at: parts.created_at,
            started_at: parts.started_at,
            finished_at: parts.finished_at,
        })
    }

    pub fn to_parts(&self) -> InstanceParts {
        InstanceParts {
            id: self.id,
            spec_id: self.spec_id,
            slug: self.slug.clone(),
            spec_version: self.spec_version,
            status: self.status,
            seats: self.seats.clone(),
            players: self.players.clone(),
            state: self.state.clone(),
            revision: self.revision,
            outcome: self.outcome.clone(),
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
        }
    }

    /// Seats `user` at `seat`. Taking the last free seat starts the game.
    /// On any error the instance is left exactly as it was.
    pub fn join(
        &mut self,
        user: UserId,
        handle: Handle,
        seat: SeatName,
        now: DateTime<Utc>,
    ) -> Result<(), LobbyError> {
        if self.status != InstanceStatus::Lobby {
            return Err(LobbyError::NotInLobby);
        }
        if !self.seats.contains(&seat) {
            return Err(LobbyError::UnknownSeat);
        }
        if self.players.iter().any(|p| p.user == user) {
            return Err(LobbyError::AlreadySeated);
        }
        if self.players.iter().any(|p| p.seat == seat) {
            return Err(LobbyError::SeatTaken);
        }

        self.players.push(Player {
            seat,
            user,
            handle,
            joined_at: now,
        });

        if self.is_full() {
            self.status = InstanceStatus::Active;
            self.started_at = Some(now);
        }
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.players.len() == self.seats.len()
    }

    /// The first seat, in the rules' order, that nobody has taken.
    pub fn first_free_seat(&self) -> Option<&SeatName> {
        self.seats
            .iter()
            .find(|seat| !self.players.iter().any(|p| &p.seat == *seat))
    }

    pub fn capacity(&self) -> usize {
        self.seats.len()
    }

    pub fn id(&self) -> InstanceId {
        self.id
    }

    pub fn spec_id(&self) -> SpecId {
        self.spec_id
    }

    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    pub fn spec_version(&self) -> u32 {
        self.spec_version
    }

    pub fn status(&self) -> InstanceStatus {
        self.status
    }

    pub fn seats(&self) -> &[SeatName] {
        &self.seats
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn state(&self) -> &Value {
        &self.state
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }
}

fn check_parts(parts: &InstanceParts) -> Result<(), DomainError> {
    let invalid = |reason: &str| Err(DomainError::InvalidInstance(reason.to_string()));

    let unique_seats: HashSet<_> = parts.seats.iter().collect();
    if parts.seats.is_empty() || unique_seats.len() != parts.seats.len() {
        return invalid("seats must be present and unique");
    }

    let seated: HashSet<_> = parts.players.iter().map(|p| &p.seat).collect();
    let users: HashSet<_> = parts.players.iter().map(|p| p.user).collect();
    let seats_known = parts.players.iter().all(|p| unique_seats.contains(&p.seat));
    if !seats_known || seated.len() != parts.players.len() || users.len() != parts.players.len() {
        return invalid("players must sit in known seats, once each");
    }

    let full = parts.players.len() == parts.seats.len();
    match parts.status {
        InstanceStatus::Lobby => {
            if full
                || parts.started_at.is_some()
                || parts.finished_at.is_some()
                || parts.outcome.is_some()
            {
                return invalid("a lobby has open seats and has not started or finished");
            }
        }
        InstanceStatus::Active => {
            if !full
                || parts.started_at.is_none()
                || parts.finished_at.is_some()
                || parts.outcome.is_some()
            {
                return invalid(
                    "an active game has every seat taken and has started, but not finished",
                );
            }
        }
        InstanceStatus::Finished => {
            if !full
                || parts.started_at.is_none()
                || parts.finished_at.is_none()
                || parts.outcome.is_none()
            {
                return invalid("a finished game has started, finished, and has an outcome");
            }
        }
    }
    Ok(())
}
