mod command;
mod error;
mod ids;
mod instance;
mod lifecycle;
mod outcome;
mod page;
mod player;
mod seat;
mod slug;
mod spec;
mod status;
mod user_game;
mod variant;

pub use command::{Command, CommandKind, MAX_POSITION_BYTES};
pub use error::{DomainError, DraftError, LobbyError, PlayError};
pub use ids::{InstanceId, SpecId, UserGameId};
pub use instance::{GameInstance, InstanceParts};
pub use lifecycle::{transition, Actor, Event, GameStatus, Refusal, MOVES};
pub use outcome::{Outcome, WinReason};
pub use page::{Colour, GamePage, PageSection, PageTheme};
pub use player::Player;
pub use seat::SeatName;
pub use slug::Slug;
pub use spec::GameSpec;
pub use status::InstanceStatus;
pub use user_game::{
    DraftContent, ShareToken, UserGame, MAX_DRAFTS_PER_AUTHOR, MAX_NAME_CHARS, MAX_RULES_BYTES, MAX_SKIN_BYTES,
};
pub use variant::Variant;
