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

// re-export inventory
pub use inventory;

#[cfg(test)]
mod tests {
    use super::*;
    use may::go;
    use std::any::Any;

    struct A;
    struct B;

    impl A {
        fn make() -> Box<dyn State> {
            Box::new(A)
        }

        fn info(&self) {
            println!("A info");
        }
    }

    impl B {
        fn make() -> Box<dyn State> {
            Box::new(B)
        }

        fn hello(&self) {
            println!("B is hello");
        }
    }

    impl State for A {
        fn state_name() -> &'static str {
            "A"
        }
        fn name(&self) -> &'static str {
            Self::state_name()
        }
        fn tear_up() -> Self {
            A
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl State for B {
        fn state_name() -> &'static str {
            "B"
        }
        fn name(&self) -> &'static str {
            Self::state_name()
        }
        fn tear_up() -> Self {
            B
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    crate::inventory::submit! {
        StateRegistration {
            id: stringify!(A),
            default_fn: A::make,
        }
    }

    crate::inventory::submit! {
        StateRegistration {
            id: stringify!(B),
            default_fn: B::make,
        }
    }

    #[test]
    fn test_state_lock() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();

        use std::sync::Arc;
        let state_lock = Arc::new(StateLock::new());
        let state_lock_1 = state_lock.clone();
        let state_lock_2 = state_lock.clone();

        go!(move || {
            let state_a1 = state_lock_2.lock::<A>().unwrap();
            state_a1.info();
        });

        go!(move || {
            let state_b = state_lock_1.lock::<B>().unwrap();
            // std::thread::sleep(std::time::Duration::from_millis(2000));
            state_b.hello();
            let state_b1 = state_lock_1.lock::<B>().unwrap();
            state_b1.hello();
        });

        println!("wait for A");
        // std::thread::sleep(std::time::Duration::from_millis(100));
        let state_a = state_lock.lock::<A>().unwrap();
        println!("wait for A done");
        state_a.info();

    }
}
