//! Reading one variable at a time. Shared by the domain files; they own the names.

use std::str::FromStr;

use super::ConfigError;

pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

pub(super) fn text(lookup: Lookup, key: &'static str, default: &str) -> String {
    lookup(key).unwrap_or_else(|| default.to_string())
}

pub(super) fn required(lookup: Lookup, key: &'static str) -> Result<String, ConfigError> {
    lookup(key).ok_or(ConfigError::Missing(key))
}

pub(super) fn parsed<T: FromStr>(
    lookup: Lookup,
    key: &'static str,
    default: T,
) -> Result<T, ConfigError> {
    match lookup(key) {
        Some(value) => value
            .parse::<T>()
            .map_err(|_| ConfigError::InvalidValue(key, value)),
        None => Ok(default),
    }
}

pub(super) fn flag(lookup: Lookup, key: &'static str, default: bool) -> Result<bool, ConfigError> {
    match lookup(key) {
        Some(value) => match value.as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(ConfigError::InvalidValue(key, value)),
        },
        None => Ok(default),
    }
}

/// A value that must be above zero; says so without repeating the value.
pub(super) fn positive(key: &'static str, value: u64) -> Result<(), ConfigError> {
    if value == 0 {
        return Err(ConfigError::InvalidValue(
            key,
            "must be greater than 0".to_string(),
        ));
    }
    Ok(())
}
