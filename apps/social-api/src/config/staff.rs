use std::collections::HashSet;

use modern_chess_kernel::UserId;

use super::{ConfigError, Lookup};

/// Who counts as staff (`STAFF_USER_IDS`): user ids, comma separated, read at start. Empty or
/// absent means nobody. The game process reads the same variable; this list is the only way to
/// become staff, and a token cannot say it.
#[derive(Debug, Clone, Default)]
pub struct StaffConfig {
    pub user_ids: HashSet<UserId>,
}

impl StaffConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let raw = lookup("STAFF_USER_IDS").unwrap_or_default();
        let mut user_ids = HashSet::new();
        for (place, entry) in raw
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .enumerate()
        {
            let id = entry.parse::<UserId>().map_err(|_| {
                // The place, not the text: the setting may sit next to secrets.
                ConfigError::InvalidValue(
                    "STAFF_USER_IDS",
                    format!("entry {} is not a user id", place + 1),
                )
            })?;
            user_ids.insert(id);
        }
        Ok(Self { user_ids })
    }
}
