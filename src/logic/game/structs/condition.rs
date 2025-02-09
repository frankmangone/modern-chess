use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ConditionDef {
    /// Type of the condition
    pub r#type: String,

    /// The actual condition, with information about what to check.
    pub check: HashMap<String, HashSet<String>>,
}
