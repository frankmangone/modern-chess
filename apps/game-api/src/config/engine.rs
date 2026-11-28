use std::time::Duration;

use modern_chess_sandbox::Limits;

use super::env::{parsed, positive};
use super::{ConfigError, Lookup};

/// What one call to the rules engine may use (`ENGINE_*`). Every game's rules run in a sandbox
/// with these limits, so rules that never end or ask for too much end one answer and not the
/// server. The defaults hold the heaviest catalogue game with a wide margin; change them with
/// care, and see `crates/sandbox`.
#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    pub limits: Limits,
}

impl EngineConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let standard = Limits::default();
        let memory_mib = parsed(lookup, "ENGINE_MEMORY_MIB", (standard.memory >> 20) as u64)?;
        let config = Self {
            limits: Limits {
                fuel: parsed(lookup, "ENGINE_FUEL", standard.fuel)?,
                memory: usize::try_from(memory_mib << 20).unwrap_or(usize::MAX),
                concurrency: parsed(lookup, "ENGINE_CONCURRENCY", standard.concurrency)?,
                patience: Duration::from_secs(parsed(
                    lookup,
                    "ENGINE_PATIENCE_SECS",
                    standard.patience.as_secs(),
                )?),
            },
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        positive("ENGINE_FUEL", self.limits.fuel)?;
        positive("ENGINE_MEMORY_MIB", (self.limits.memory >> 20) as u64)?;
        positive("ENGINE_CONCURRENCY", self.limits.concurrency as u64)?;
        positive("ENGINE_PATIENCE_SECS", self.limits.patience.as_secs())?;
        Ok(())
    }
}
