//! Zones declared under a spec's `zones`, and whether a place is in one.

use std::collections::{HashMap, HashSet};

use crate::logic::Game;
use crate::shared::Position;

/// A zone: for each player, the names of the places in their part of it.
#[derive(Clone, Debug, Default)]
pub struct Zone {
    pub places: HashMap<String, HashSet<String>>,
}

impl Game {
    /// Whether `position` is in `player`'s part of the zone named `zone`. A zone the game does
    /// not have holds nowhere.
    pub fn in_zone(&self, position: &Position, zone: &str, player: &str) -> bool {
        self.rules
            .zones
            .get(zone)
            .and_then(|zone| zone.places.get(player))
            .is_some_and(|places| places.contains(position.as_str()))
    }
}
