mod command;
mod error;
mod ids;
mod instance;
mod lifecycle;
mod outcome;
mod page;
mod player;
mod publishing;
mod record;
mod seat;
mod slug;
mod spec;
mod staff;
mod status;
mod user_game;
mod variant;

pub use command::{Command, CommandKind, MAX_POSITION_BYTES};
pub use error::{DomainError, DraftError, LobbyError, PlayError};
pub use ids::{InstanceId, SpecId, UserGameId};
pub use instance::{new_seed, GameInstance, InstanceParts, Opening};
pub use lifecycle::{transition, Actor, Event, GameStatus, Refusal, MOVES};
pub use outcome::{Outcome, WinReason};
pub use page::{Colour, GamePage, PageSection, PageTheme};
pub use player::Player;
pub use publishing::{
    check_players, is_reserved, slug_base, slug_try, Listing, LICENCE, MAX_DESCRIPTION_CHARS,
    MAX_PLAYERS, MAX_TAGLINE_CHARS, SORT_ORDER,
};
pub use record::{result_for, Record, SeatResult};
pub use seat::SeatName;
pub use slug::Slug;
pub use spec::GameSpec;
pub use staff::Staff;
pub use status::InstanceStatus;
pub use user_game::{
    DraftContent, ShareToken, UserGame, MAX_DRAFTS_PER_AUTHOR, MAX_NAME_CHARS, MAX_RULES_BYTES,
    MAX_SKIN_BYTES,
};
pub use variant::Variant;
