use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::PositionSpec;
use crate::shared::{Coordinates, POSITION, STATE};

#[derive(Clone, Debug, Serialize)]
pub struct ConditionSpec {
    /// Unique identifier for this condition
    pub code: String,
    /// Type of the condition
    pub r#type: String,

    #[serde(deserialize_with = "deserialize_check")]
    pub check: HashMap<String, HashSet<String>>,

    /// For POSITION: the first entry written as coordinates rather than as a name, if any. A
    /// map refuses coordinates.
    #[serde(skip)]
    pub written_coordinates: Option<Coordinates>,

    /// For POSITION: the entries written as names rather than as coordinates, sorted. A grid
    /// refuses a name that is not one of its squares.
    #[serde(skip)]
    pub written_names: Vec<String>,
}

impl<'de> Deserialize<'de> for ConditionSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct Helper {
            code: String,
            r#type: String,
            check: Value,
        }

        let helper = Helper::deserialize(deserializer)?;

        // Now we can use the type to determine how to deserialize check
        let mut written_coordinates = None;
        let mut written_names = Vec::new();
        let check: HashMap<String, HashSet<String>> = match helper.r#type.as_str() {
            POSITION => {
                let raw_map: HashMap<String, Vec<PositionSpec>> =
                    serde_json::from_value(helper.check).map_err(de::Error::custom)?;

                raw_map
                    .into_iter()
                    .map(|(team, positions)| {
                        let position_strings: HashSet<String> = positions
                            .iter()
                            .map(|pos| {
                                match pos {
                                    PositionSpec::Coordinates(coordinates) => {
                                        written_coordinates
                                            .get_or_insert_with(|| coordinates.clone());
                                    }
                                    PositionSpec::Name(name) => written_names.push(name.clone()),
                                }
                                pos.position().to_string()
                            })
                            .collect();
                        (team, position_strings)
                    })
                    .collect()
            }
            STATE => {
                let raw_map: HashMap<String, Vec<String>> =
                    serde_json::from_value(helper.check).map_err(de::Error::custom)?;

                raw_map
                    .into_iter()
                    .map(|(team, states)| (team, states.into_iter().collect()))
                    .collect()
            }
            _ => return Err(de::Error::custom("Unknown condition type")),
        };

        written_names.sort();
        Ok(ConditionSpec {
            code: helper.code,
            r#type: helper.r#type,
            check,
            written_coordinates,
            written_names,
        })
    }
}

/// A zone: for each player, the places that make up their part of it, written as coordinates
/// on a grid or as names anywhere.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneSpec {
    pub name: String,
    pub places: HashMap<String, Vec<PositionSpec>>,
}
