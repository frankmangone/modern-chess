// A panic in the engine freezes the board in the browser, so outside of tests the engine does
// not unwrap, expect or index: it returns an error or skips the case instead.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)
)]

/// The position of a grid square, by its coordinates: `pos![4, 2]` is the square named `"4,2"`.
#[macro_export]
macro_rules! pos {
    () => {
        $crate::shared::Position::default()
    };
    ($($coordinate:expr),+ $(,)?) => {
        $crate::shared::Position::from_coordinates(&[$($coordinate),+])
    };
}

pub mod logic;
pub mod shared;
pub mod specs;
#[cfg(test)]
mod tests;
