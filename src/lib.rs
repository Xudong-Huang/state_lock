//! `StateLock` provide a sync primitive that can be used to wait for a state to be ready.
//!
//! The state type must impl `State` trait, which contains the tear up and tear down logic
//! to prepare and destroy the state. They you can call the `StateLock::lock` to obtain the state.
//!
//! Multi thread could call `StateLock::lock` at the same time. if the state is ready, the thread
//! would not block, else block until the state is ready.

#[macro_use]
extern crate log;

mod state;
pub use state::State;

mod lock;
pub use lock::{StateGuard, StateLock};

mod registry;
pub use registry::StateRegistration;

// re-export #[derive(State)] for convenience
pub use state_derive::State;

// re-export inventory
pub use inventory;
