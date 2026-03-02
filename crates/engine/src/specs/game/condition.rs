use std::collections::{HashMap, HashSet};
use serde::{de, Deserialize, Serialize, Deserializer};
use serde_json::Value;

use crate::shared::{Position, into_string, POSITION, STATE};

#[derive(Clone, Debug, Serialize)]
pub struct ConditionSpec {
    /// Unique identifier for this condition
    pub code: String,
    /// Type of the condition
    pub r#type: String,

    #[serde(deserialize_with = "deserialize_check")]
    pub check: HashMap<String, HashSet<String>>,
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
        let check: HashMap<String, HashSet<String>> = match helper.r#type.as_str() {
            POSITION => {
                let raw_map: HashMap<String, Vec<Position>> = 
                    serde_json::from_value(helper.check).map_err(de::Error::custom)?;
                
                raw_map.into_iter()
                    .map(|(team, positions)| {
                        let position_strings: HashSet<String> = positions.iter()
                            .map(|pos| into_string(pos))
                            .collect();
                        (team, position_strings)
                    })
                    .collect()
            },
            STATE => {
                let raw_map: HashMap<String, Vec<String>> = 
                    serde_json::from_value(helper.check).map_err(de::Error::custom)?;
                
                raw_map.into_iter()
                    .map(|(team, states)| {
                        (team, states.into_iter().collect())
                    })
                    .collect()
            },
            _ => return Err(de::Error::custom("Unknown condition type")),
        };

        Ok(ConditionSpec {
            code: helper.code,
            r#type: helper.r#type,
            check,
        })
    }
}
