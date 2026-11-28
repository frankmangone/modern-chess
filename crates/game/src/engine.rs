//! The only door to the rules engine.
//!
//! Nothing in this crate runs a game's rules itself. Every question (can the rules be read, what
//! are the seats, what does the board look like, what does this move do) goes through [`Engine`],
//! which sends it to the sandbox (`modern-chess-sandbox`): the engine built to WebAssembly, run
//! with a fuel budget and a memory cap, so rules that never end, or that fault, cost one answer
//! and not the server. That is what makes it safe to run rules written by strangers.
//!
//! Rules and seats are immutable once stored, so what only they decide (a game's seats, the
//! picture of its board) is remembered by the rules' hash and asked once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use modern_chess_sandbox::{Limits, Sandbox};
use modern_chess_sandbox_protocol::{Request, Response};
use serde_json::Value;
use thiserror::Error;
use tracing::{debug, warn};

pub use modern_chess_sandbox_protocol::{Problem, Seeing, Standing, Step};

use crate::{new_seed, GameSpec, Opening, SeatName};

/// How many remembered answers of each kind are kept before the memory is cleared.
const REMEMBERED: usize = 4096;

/// Why the engine gave no answer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    /// The engine will not read these rules, or this state does not fit them.
    #[error("the engine cannot use these rules: {0}")]
    Unusable(String),
    /// The engine stopped: a fault, a trap, the time or memory budget, or too many waiting.
    /// The cause is for the log, never for a player.
    #[error("the engine stopped: {0}")]
    Stopped(String),
}

/// A seat's move, as the engine answers it.
#[derive(Debug, Clone, PartialEq)]
pub enum Move {
    /// It is not that seat's turn to answer.
    NotYourTurn,
    /// The game refused it. The reason may quote the engine: for the log.
    Rejected(String),
    Played {
        state: Value,
        standing: Standing,
        /// What the seat sees after it; everyone sees it all once the match is over.
        view: Value,
    },
}

/// What the rules are, in a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub name: String,
    pub players: Vec<SeatName>,
}

#[derive(Clone)]
enum How {
    Sandboxed,
    /// The engine in this process. For tests that need no sandbox; never for rules a stranger wrote.
    Unsandboxed,
}

/// The rules engine, as the server uses it. Cheap to clone; clones share what is remembered.
#[derive(Clone)]
pub struct Engine {
    how: How,
    sandbox: Arc<OnceLock<Result<Sandbox, String>>>,
    limits: Limits,
    seats: Arc<Mutex<HashMap<String, Vec<SeatName>>>>,
    pictures: Arc<Mutex<HashMap<String, Value>>>,
}

/// One sandbox for the whole process, made when first needed: compiling the engine takes a
/// moment, and every `Engine` made with the default limits can share it.
fn shared() -> &'static OnceLock<Result<Sandbox, String>> {
    static SHARED: OnceLock<Result<Sandbox, String>> = OnceLock::new();
    &SHARED
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// The engine in the sandbox, with the default budget.
    pub fn new() -> Self {
        Self {
            how: How::Sandboxed,
            sandbox: Arc::new(OnceLock::new()),
            limits: Limits::default(),
            seats: Arc::default(),
            pictures: Arc::default(),
        }
    }

    /// The engine in the sandbox with another budget (the server's configuration, or a test
    /// that wants to see the budget run out).
    pub fn with_limits(limits: Limits) -> Self {
        Self {
            limits,
            ..Self::new()
        }
    }

    /// The engine in this process, with no sandbox: **for tests, and only for rules a person on
    /// this team wrote.** A game's rules from anyone else must never reach it.
    #[doc(hidden)]
    pub fn unsandboxed() -> Self {
        Self {
            how: How::Unsandboxed,
            ..Self::new()
        }
    }

    /// Compiles the sandbox now, so a server that cannot run it says so at start.
    pub fn warm_up(&self) -> Result<(), EngineError> {
        self.sandbox().map(|_| ())
    }

    fn sandbox(&self) -> Result<&Sandbox, EngineError> {
        let made = if self.limits == Limits::default() {
            shared().get_or_init(|| Sandbox::new(self.limits).map_err(|err| err.to_string()))
        } else {
            self.sandbox
                .get_or_init(|| Sandbox::new(self.limits).map_err(|err| err.to_string()))
        };
        made.as_ref()
            .map_err(|err| EngineError::Stopped(format!("the sandbox could not start: {err}")))
    }

    async fn ask(&self, request: Request) -> Result<Response, EngineError> {
        let response = match self.how {
            #[allow(
                clippy::disallowed_methods,
                reason = "the unsandboxed engine is for tests"
            )]
            How::Unsandboxed => modern_chess_sandbox_protocol::answer(request),
            How::Sandboxed => {
                let sandbox = self.sandbox()?;
                match sandbox.call(&request).await {
                    Ok((response, cost)) => {
                        debug!(fuel = cost.fuel, "engine call");
                        if cost.fuel > sandbox.limits().fuel / 4 {
                            warn!(
                                fuel = cost.fuel,
                                "an engine call used a quarter of its budget"
                            );
                        }
                        response
                    }
                    Err(err) => return Err(EngineError::Stopped(err.to_string())),
                }
            }
        };
        match response {
            Response::Failed { message } => Err(EngineError::Unusable(message)),
            Response::Unreadable(problem) => Err(EngineError::Unusable(problem.message)),
            other => Ok(other),
        }
    }

    /// Reads rules the way a game would be opened and says what they are, or why not.
    pub async fn check(&self, rules: &Value) -> Result<Summary, EngineError> {
        match self
            .ask(Request::Check {
                rules: rules.to_string(),
            })
            .await?
        {
            Response::Checked { name, players } => Ok(Summary {
                name,
                players: players.into_iter().map(SeatName::new).collect(),
            }),
            other => Err(unexpected(&other)),
        }
    }

    /// The seats of `spec`, in the order its rules list their players. Asked once per rules.
    pub async fn seats(&self, spec: &GameSpec) -> Result<Vec<SeatName>, EngineError> {
        if let Some(known) = remembered(&self.seats, &spec.rules_hash) {
            return Ok(known);
        }
        let seats = self.check(&spec.rules).await?.players;
        remember(&self.seats, &spec.rules_hash, seats.clone());
        Ok(seats)
    }

    /// The opening position of a new match of `spec`, with the seed of its own chance.
    pub async fn open(&self, spec: &GameSpec) -> Result<Opening, EngineError> {
        let request = Request::Open {
            rules: spec.rules.to_string(),
            seed: new_seed(),
        };
        match self.ask(request).await? {
            Response::Opened { seats, state } => Ok(Opening {
                seats: seats.into_iter().map(SeatName::new).collect(),
                state,
            }),
            other => Err(unexpected(&other)),
        }
    }

    /// The board of `spec` before anyone moves, as a spectator sees it: the picture of the game
    /// on its card. Asked once per rules.
    pub async fn picture(&self, spec: &GameSpec) -> Result<Value, EngineError> {
        if let Some(known) = remembered(&self.pictures, &spec.rules_hash) {
            return Ok(known);
        }
        let request = Request::Picture {
            rules: spec.rules.to_string(),
        };
        match self.ask(request).await? {
            Response::Picture { view } => {
                remember(&self.pictures, &spec.rules_hash, view.clone());
                Ok(view)
            }
            other => Err(unexpected(&other)),
        }
    }

    /// A match as `seeing` sees it.
    pub async fn view(
        &self,
        spec: &GameSpec,
        state: &Value,
        revision: u64,
        seeing: Seeing,
    ) -> Result<Value, EngineError> {
        let request = Request::View {
            rules: spec.rules.to_string(),
            state: state.clone(),
            revision,
            seeing,
        };
        match self.ask(request).await? {
            Response::Viewed { view } => Ok(view),
            other => Err(unexpected(&other)),
        }
    }

    /// `seat`'s move in a match.
    pub async fn play(
        &self,
        spec: &GameSpec,
        state: &Value,
        revision: u64,
        seat: &SeatName,
        step: Step,
    ) -> Result<Move, EngineError> {
        let request = Request::Play {
            rules: spec.rules.to_string(),
            state: state.clone(),
            revision,
            seat: seat.as_str().to_string(),
            step,
        };
        match self.ask(request).await? {
            Response::NotYourTurn => Ok(Move::NotYourTurn),
            Response::Rejected { reason } => Ok(Move::Rejected(reason)),
            Response::Played {
                state,
                standing,
                view,
            } => Ok(Move::Played {
                state,
                standing,
                view,
            }),
            other => Err(unexpected(&other)),
        }
    }
}

fn unexpected(response: &Response) -> EngineError {
    EngineError::Stopped(format!("the engine answered something else: {response:?}"))
}

fn remembered<T: Clone>(memory: &Mutex<HashMap<String, T>>, key: &str) -> Option<T> {
    memory.lock().ok()?.get(key).cloned()
}

fn remember<T>(memory: &Mutex<HashMap<String, T>>, key: &str, value: T) {
    if let Ok(mut memory) = memory.lock() {
        if memory.len() >= REMEMBERED {
            memory.clear();
        }
        memory.insert(key.to_string(), value);
    }
}

/// The opening of a new match of `spec`, asked of the engine in this process, on this thread.
/// **For tests only**, for rules a person on this team wrote: it has no sandbox.
#[doc(hidden)]
#[allow(
    clippy::disallowed_methods,
    reason = "for tests, which have no sandbox"
)]
pub fn opening_for_tests(spec: &GameSpec) -> Opening {
    match modern_chess_sandbox_protocol::answer(Request::Open {
        rules: spec.rules.to_string(),
        seed: new_seed(),
    }) {
        Response::Opened { seats, state } => Opening {
            seats: seats.into_iter().map(SeatName::new).collect(),
            state,
        },
        other => panic!("the engine would not open the test's rules: {other:?}"),
    }
}
