//! Runs the rules engine for the game server, so a game's rules can never take the server down.
//!
//! The engine is built to WebAssembly (`modern-chess-sandbox-guest`) with no imports at all and
//! run here with `wasmtime`. Every call gets a fresh instance with a **fuel budget** (a
//! deterministic count of instructions: a rule that never ends runs out and traps, whatever the
//! machine) and a **memory cap**, and nothing is kept between calls. A trap, a stack overflow or
//! an exhausted budget is an [`Error`] the caller turns into an answer; the server carries on.
//! That is what makes it safe to run rules written by strangers.

use std::sync::Arc;
use std::time::Duration;

use modern_chess_sandbox_protocol::{Request, Response};
use thiserror::Error;
use tokio::sync::Semaphore;
use wasmtime::{
    Config, Engine, InstancePre, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

/// The engine as a module, built by `build.rs`.
static GUEST: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"));

/// What one call may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Instructions, roughly (wasmtime's fuel). The heaviest call a catalogue game makes costs
    /// under 100 million (`tests/sandbox.rs` prints the table), so this is more than twenty times
    /// that, and a call that never ends is cut off in about a second.
    pub fuel: u64,
    /// The most memory the module may grow to, in bytes.
    pub memory: usize,
    /// Calls that run at once; the rest wait their turn.
    pub concurrency: usize,
    /// The longest a call may wait for its turn before it is turned away.
    pub patience: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel: 2_000_000_000,
            memory: 128 << 20,
            concurrency: std::thread::available_parallelism().map_or(4, usize::from),
            patience: Duration::from_secs(5),
        }
    }
}

/// Why a call did not give an answer.
#[derive(Debug, Error)]
pub enum Error {
    /// The call used up its fuel: the rules (or the position) cost more than a move may.
    #[error("the rules ran out of their time budget")]
    OutOfFuel,
    /// The module trapped (a panic, a stack overflow, memory beyond the cap).
    #[error("the engine stopped: {0}")]
    Trapped(String),
    /// Too many calls were waiting for the engine.
    #[error("the engine is busy")]
    Busy,
    /// The module or the host broke the protocol; a bug, not a game.
    #[error("sandbox fault: {0}")]
    Fault(String),
}

/// What a finished call cost, for calibration and metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub fuel: u64,
}

struct Host {
    limits: StoreLimits,
}

/// The sandbox: one compiled module, shared by every call.
#[derive(Clone)]
pub struct Sandbox {
    inner: Arc<Inner>,
}

struct Inner {
    engine: Engine,
    module: InstancePre<Host>,
    limits: Limits,
    turns: Arc<Semaphore>,
}

impl Sandbox {
    /// Compiles the module. Takes a moment (done once, at start).
    pub fn new(limits: Limits) -> Result<Self, Error> {
        let mut config = Config::new();
        config.consume_fuel(true);
        // Threads and shared memory are not even compiled in (see Cargo.toml): the engine needs
        // neither, and less is less to get wrong.
        let engine = Engine::new(&config).map_err(fault)?;
        let module = Module::new(&engine, GUEST).map_err(fault)?;
        // No functions are offered to the module: it imports none, and one that did would not link.
        let linker = Linker::<Host>::new(&engine);
        let module = linker.instantiate_pre(&module).map_err(fault)?;
        Ok(Self {
            inner: Arc::new(Inner {
                engine,
                module,
                turns: Arc::new(Semaphore::new(limits.concurrency.max(1))),
                limits,
            }),
        })
    }

    /// Asks the engine one question, off the async runtime's threads, after waiting its turn.
    pub async fn call(&self, request: &Request) -> Result<(Response, Cost), Error> {
        let text = serde_json::to_string(request).map_err(fault)?;
        let turn = tokio::time::timeout(
            self.inner.limits.patience,
            Arc::clone(&self.inner.turns).acquire_owned(),
        )
        .await
        .map_err(|_| Error::Busy)?
        .map_err(fault)?;
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let _turn = turn;
            inner.run(&text)
        })
        .await
        .map_err(fault)?
    }

    /// The same, on the calling thread, for tests and tools that are not async.
    pub fn call_blocking(&self, request: &Request) -> Result<(Response, Cost), Error> {
        let text = serde_json::to_string(request).map_err(fault)?;
        self.inner.run(&text)
    }

    /// The limits this sandbox was made with.
    pub fn limits(&self) -> Limits {
        self.inner.limits
    }
}

impl Inner {
    fn run(&self, text: &str) -> Result<(Response, Cost), Error> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory)
            .instances(1)
            .memories(1)
            .tables(4)
            .table_elements(100_000)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&self.engine, Host { limits });
        store.limiter(|host| &mut host.limits);
        store.set_fuel(self.limits.fuel).map_err(fault)?;

        let answer = self.exchange(&mut store, text)?;
        let used = self
            .limits
            .fuel
            .saturating_sub(store.get_fuel().unwrap_or(0));
        let response = serde_json::from_slice(&answer)
            .map_err(|err| Error::Fault(format!("the module's answer is not a response: {err}")))?;
        Ok((response, Cost { fuel: used }))
    }

    fn exchange(&self, store: &mut Store<Host>, text: &str) -> Result<Vec<u8>, Error> {
        let missing = |what: &str| Error::Fault(format!("the module has no {what}"));
        let instance = self.module.instantiate(&mut *store).map_err(stopped)?;
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| missing("memory"))?;
        let alloc = instance
            .get_typed_func::<u32, u32>(&mut *store, "alloc")
            .map_err(|_| missing("alloc"))?;
        let handle = instance
            .get_typed_func::<(u32, u32), u64>(&mut *store, "handle")
            .map_err(|_| missing("handle"))?;

        let length =
            u32::try_from(text.len()).map_err(|_| Error::Fault("request too big".into()))?;
        let at = alloc.call(&mut *store, length).map_err(stopped)?;
        memory
            .write(&mut *store, at as usize, text.as_bytes())
            .map_err(|err| Error::Fault(format!("writing the request: {err}")))?;
        let packed = handle.call(&mut *store, (at, length)).map_err(stopped)?;

        let (from, size) = ((packed >> 32) as usize, (packed & 0xffff_ffff) as usize);
        memory
            .data(&*store)
            .get(from..from.saturating_add(size))
            .map(<[u8]>::to_vec)
            .ok_or_else(|| Error::Fault("the answer is outside the module's memory".into()))
    }
}

fn fault(err: impl std::fmt::Display) -> Error {
    Error::Fault(err.to_string())
}

/// A trap as the caller should hear it: the budget running out is its own thing.
fn stopped(err: wasmtime::Error) -> Error {
    match err.downcast_ref::<wasmtime::Trap>() {
        Some(wasmtime::Trap::OutOfFuel) => Error::OutOfFuel,
        _ => Error::Trapped(format!("{err:#}")),
    }
}
