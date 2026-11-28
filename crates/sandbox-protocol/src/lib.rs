//! What the game server asks of the rules engine, as data in and data out.
//!
//! The server never calls the engine itself: it sends a [`Request`] and reads a [`Response`].
//! [`handle`] is the one function that answers, and it runs in two places: inside the
//! WebAssembly sandbox (`modern-chess-sandbox-guest`, which is what a live server uses), and in
//! this process (tests, and the check that both give the same answers). A request carries
//! everything it needs, because nothing is kept between calls: the rules, and for a match its
//! saved state, so a call that runs away or faults leaves nothing half done.

use modern_chess_session::{Answer, Position, Session, Status, Viewer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One question for the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Can the engine read these rules, and what game are they?
    Check { rules: String },
    /// The opening position of a match: its seats, and its saved state. `seed` is the seed of the
    /// match's chance (dice, shuffles); a game with none ignores it.
    Open { rules: String, seed: u64 },
    /// The board before anyone moves, as a spectator sees it: the picture of the game on its card.
    Picture { rules: String },
    /// A match as `seeing` sees it.
    View {
        rules: String,
        state: Value,
        revision: u64,
        seeing: Seeing,
    },
    /// A seat's move.
    Play {
        rules: String,
        state: Value,
        revision: u64,
        seat: String,
        step: Step,
    },
}

/// Who is looking: what a view shows depends on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "who", rename_all = "snake_case")]
pub enum Seeing {
    /// Everything: a match that is over, shown to all alike.
    Everyone,
    /// Someone watching who is no player.
    Spectator,
    /// A seat's player.
    Player { seat: String },
}

/// What a seat asks the game to do. At most one of the answers is given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    Choose {
        place: Option<String>,
        code: Option<String>,
        held: Option<String>,
        action: Option<String>,
    },
    /// Takes back the answers given so far this turn.
    Cancel,
}

/// How a match stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "standing", rename_all = "snake_case")]
pub enum Standing {
    Playing,
    Draw,
    Won { winner: String },
}

/// Why the engine will not read rules, with the place in the text when the text is at fault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub path: Option<String>,
}

/// The engine's answer. `View`s are the same JSON a front end draws, passed through as it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case")]
pub enum Response {
    Checked {
        name: String,
        players: Vec<String>,
    },
    Unreadable(Problem),
    Opened {
        seats: Vec<String>,
        state: Value,
    },
    Picture {
        view: Value,
    },
    Viewed {
        view: Value,
    },
    /// It is not that seat's turn to answer.
    NotYourTurn,
    /// The game refused the move; the reason may quote the engine and is for the log, never a player.
    Rejected {
        reason: String,
    },
    Played {
        state: Value,
        standing: Standing,
        /// The game as the seat sees it after the move; everyone sees it all once it has ended.
        view: Value,
    },
    /// The engine could not do what was asked (rules it will not read, a state that does not fit).
    Failed {
        message: String,
    },
}

/// Answers one request, given as JSON text, with the response as JSON text. This is all the
/// sandbox exposes: text in, text out.
pub fn handle(request: &str) -> String {
    let response = match serde_json::from_str::<Request>(request) {
        Ok(request) => answer(request),
        Err(err) => Response::Failed {
            message: format!("not a request: {err}"),
        },
    };
    // A response is plain data, so writing it cannot fail; the fallback is only for the compiler.
    serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"answer":"failed","message":"unwritable response"}"#.to_string())
}

/// Answers one request.
pub fn answer(request: Request) -> Response {
    match request {
        Request::Check { rules } => match modern_chess_session::check_spec(&rules) {
            Ok(summary) => Response::Checked {
                name: summary.name,
                players: summary.players,
            },
            Err(problem) => Response::Unreadable(Problem {
                message: problem.message,
                line: problem.line,
                column: problem.column,
                path: problem.path,
            }),
        },
        Request::Open { rules, seed } => open(&rules, seed),
        Request::Picture { rules } => match Session::new(&rules) {
            Ok(session) => {
                let mut view = session.view_for(&Viewer::Spectator);
                // Nobody is choosing anything in a picture.
                view.choice = Default::default();
                Response::Picture {
                    view: to_value(&view),
                }
            }
            Err(message) => Response::Failed { message },
        },
        Request::View {
            rules,
            state,
            revision,
            seeing,
        } => match Session::resume(&rules, &state, revision) {
            Ok(session) => Response::Viewed {
                view: to_value(&session.view_for(&viewer(&seeing))),
            },
            Err(message) => Response::Failed { message },
        },
        Request::Play {
            rules,
            state,
            revision,
            seat,
            step,
        } => play(&rules, &state, revision, &seat, step),
    }
}

fn open(rules: &str, seed: u64) -> Response {
    let session = match Session::new_with_seed(rules, seed) {
        Ok(session) => session,
        Err(message) => return Response::Failed { message },
    };
    let view = session.view_for(&Viewer::Everyone);
    Response::Opened {
        seats: view.players,
        state: session.state(),
    }
}

fn play(rules: &str, state: &Value, revision: u64, seat: &str, step: Step) -> Response {
    let mut session = match Session::resume(rules, state, revision) {
        Ok(session) => session,
        Err(message) => return Response::Failed { message },
    };
    if !session.is_to_move(seat) {
        return Response::NotYourTurn;
    }
    let by = Some(seat);
    let applied = match step {
        Step::Choose {
            place,
            code,
            held,
            action,
        } => match (place, code, held, action) {
            // A name no board has used is no place.
            (Some(place), None, None, None) => match Position::existing(&place) {
                Some(position) => session.choose_as(by, Answer::Place(position)),
                None => Err("that square is not on the board".to_string()),
            },
            (None, Some(code), None, None) => session.choose_as(by, Answer::Code(code)),
            (None, None, Some(held), None) => session.choose_as(by, Answer::Held(held)),
            (None, None, None, Some(action)) => session.choose_as(by, Answer::Action(action)),
            _ => Err("a choice needs exactly one answer".to_string()),
        },
        Step::Cancel => session.cancel_as(by),
    };
    if let Err(reason) = applied {
        return Response::Rejected { reason };
    }
    let standing = match session.view_for(&Viewer::Spectator).status {
        Status::Playing => Standing::Playing,
        Status::Draw => Standing::Draw,
        Status::Won { winner } => Standing::Won { winner },
    };
    let seeing = match standing {
        Standing::Playing => Viewer::Player(seat.to_string()),
        _ => Viewer::Everyone,
    };
    Response::Played {
        state: session.state(),
        standing,
        view: to_value(&session.view_for(&seeing)),
    }
}

fn viewer(seeing: &Seeing) -> Viewer {
    match seeing {
        Seeing::Everyone => Viewer::Everyone,
        Seeing::Spectator => Viewer::Spectator,
        Seeing::Player { seat } => Viewer::Player(seat.clone()),
    }
}

fn to_value(view: &modern_chess_session::View) -> Value {
    serde_json::to_value(view).unwrap_or(Value::Null)
}
