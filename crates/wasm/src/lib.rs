use wasm_bindgen::prelude::*;

use modern_chess_session::{Answer, Session};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;
}

/// The seed of a new board's chance (dice, shuffles): the browser's own randomness, so the page
/// has nothing to pass. Anywhere else (the tests of this crate) the clock stands in.
fn fresh_seed() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // `random()` is below 1, so each product is below 2^32 and the casts lose nothing.
        let high = (random() * 4_294_967_296.0) as u64;
        let low = (random() * 4_294_967_296.0) as u64;
        high << 32 | low
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64)
    }
}

/// A panic in the engine ends the call with a bare "unreachable". Say what happened in the
/// browser console first, so a report of a frozen board comes with a cause.
#[wasm_bindgen(start)]
fn report_panics() {
    std::panic::set_hook(Box::new(|info| error(&format!("engine panic: {info}"))));
}

/// An answer from its JSON text; anything else is refused.
fn parse_answer(text: &str) -> Result<Answer, String> {
    serde_json::from_str(text).map_err(|_| "that is not an answer".to_string())
}

/// The JavaScript-facing game. Every method delegates to `Session`; errors become
/// JavaScript exceptions carrying the engine's message.
#[wasm_bindgen]
pub struct Board {
    session: Session,
}

#[wasm_bindgen]
impl Board {
    #[wasm_bindgen(constructor)]
    pub fn new(spec_json: &str) -> Result<Board, JsError> {
        Session::new_with_seed(spec_json, fresh_seed())
            .map(|session| Board { session })
            .map_err(|e| JsError::new(&e))
    }

    /// The current snapshot, as JSON text.
    pub fn view(&self) -> String {
        serde_json::to_string(&self.session.view()).expect("a view always serializes")
    }

    /// Answers the next choice. `answer` is an answer's JSON: `{"place":"4,2"}`,
    /// `{"code":"QUEEN"}`, `{"held":"PAWN"}` or `{"action":"PASS"}`.
    pub fn choose(&mut self, answer: &str) -> Result<(), JsError> {
        let answer = parse_answer(answer).map_err(|e| JsError::new(&e))?;
        self.session.choose(answer).map_err(|e| JsError::new(&e))
    }

    /// Takes back the answers given so far this turn.
    pub fn cancel(&mut self) -> Result<(), JsError> {
        self.session.cancel().map_err(|e| JsError::new(&e))
    }

    pub fn reset(&mut self) {
        self.session.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_one_of_four_keys_with_a_string() {
        assert_eq!(
            parse_answer(r#"{"place":"4,2"}"#),
            Ok(Answer::Place("4,2".into()))
        );
        assert_eq!(
            parse_answer(r#"{"code":"QUEEN"}"#),
            Ok(Answer::Code("QUEEN".into()))
        );
        assert_eq!(
            parse_answer(r#"{"held":"PAWN"}"#),
            Ok(Answer::Held("PAWN".into()))
        );
        assert_eq!(
            parse_answer(r#"{"action":"PASS"}"#),
            Ok(Answer::Action("PASS".into()))
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in [
            "",
            "4,1",
            "{}",
            r#"{"place":4}"#,
            r#"{"nothing":"4,1"}"#,
            r#"{"place":"4,1","code":"QUEEN"}"#,
        ] {
            assert!(parse_answer(text).is_err(), "{text}");
        }
    }
}
