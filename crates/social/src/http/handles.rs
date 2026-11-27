use axum::extract::{Path, State};
use axum::Json;
use modern_chess_kernel::CurrentUser;
use serde::Serialize;

use super::error::ApiError;
use super::SocialState;
use crate::handles::choose;

#[derive(Serialize)]
pub struct Problem {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
pub struct Availability {
    available: bool,
    /// Why the name cannot be used: it is not allowed, or it is taken.
    problem: Option<Problem>,
}

/// `GET /handles/{name}`: whether a username can be chosen, so the set-up can say so while the
/// person types. It needs a token, which keeps it from being a way to list who is here.
pub async fn check(
    State(state): State<SocialState>,
    _caller: CurrentUser,
    Path(name): Path<String>,
) -> Result<Json<Availability>, ApiError> {
    let handle = match choose(&name) {
        Ok(handle) => handle,
        Err(problem) => {
            return Ok(Json(Availability {
                available: false,
                problem: Some(Problem {
                    code: problem.code(),
                    message: problem.message(),
                }),
            }))
        }
    };
    let free = state
        .users
        .handle_is_free(&handle)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(Availability {
        available: free,
        problem: (!free).then_some(Problem {
            code: "taken",
            message: "That name is taken. Please choose another.",
        }),
    }))
}
