// A panic in the engine freezes the board in the browser, so outside of tests the engine does
// not unwrap, expect or index: it returns an error or skips the case instead.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)
)]

pub mod logic;
pub mod shared;
pub mod specs;
pub mod tests;
