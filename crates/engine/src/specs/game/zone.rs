use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::PositionSpec;

/// A zone: for each player, the places that make up their part of it, written as coordinates
/// on a grid or as names anywhere.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneSpec {
    pub name: String,
    pub places: HashMap<String, Vec<PositionSpec>>,
}
