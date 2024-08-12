use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BoardSpec {
    dimensions: Vec<u8>
}
