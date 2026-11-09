use wasm_bindgen::prelude::*;

use modern_chess_session::Session;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

/// A panic in the engine ends the call with a bare "unreachable". Say what happened in the
/// browser console first, so a report of a frozen board comes with a cause.
#[wasm_bindgen(start)]
fn report_panics() {
    std::panic::set_hook(Box::new(|info| error(&format!("engine panic: {info}"))));
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
        Session::new(spec_json)
            .map(|session| Board { session })
            .map_err(|e| JsError::new(&e))
    }

    /// The current snapshot, as JSON text.
    pub fn view(&self) -> String {
        serde_json::to_string(&self.session.view()).expect("a view always serializes")
    }

    pub fn select(&mut self, x: u8, y: u8) -> Result<(), JsError> {
        self.session.select(x, y).map_err(|e| JsError::new(&e))
    }

    pub fn make_move(&mut self, x: u8, y: u8) -> Result<(), JsError> {
        self.session.make_move(x, y).map_err(|e| JsError::new(&e))
    }

    pub fn promote(&mut self, code: &str) -> Result<(), JsError> {
        self.session.promote(code).map_err(|e| JsError::new(&e))
    }

    pub fn reset(&mut self) {
        self.session.reset();
    }
}
