//! `StateLock` provide a sync primitive that can be used to wait for a state to be ready.
//!
//! The state type must impl `State` trait, which contains the tear up and tear down logic.
//! The tear up is normally implemented by using the `Default::default` to create the state.
//! And for tear down, `drop` is usually enough, the state will gone if no task refs the state.
//! You can call the `StateLock::lock` or `StateLock::lock_by_state_name` to obtain a state.
//!
//! Multi thread could call `StateLock::lock` or `StateLock::lock_by_state_name` at the same time.
//! If the state is ready, the thread would not block, else block until the state is ready.

#[macro_use]
extern crate log;

pub mod default;

mod state;
pub use state::{RawState, State, StateGuard};

mod lock;
pub use lock::StateLock;

mod registry;
pub use registry::{StateRegistration, STATE_REGISTRATION};

// re-export #[derive(State)] for convenience
pub use state_derive::State;

// re-export linkme
pub use intertrait::linkme;

// re-export intertrait
pub use intertrait;
