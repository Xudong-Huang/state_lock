//! `StateLock` provide a sync primitive that can be used to wait for a state to be ready.
//!
//! The state type must impl `State` trait, which contains the tear up and tear down logic
//! to prepare and destroy the state. They you can call the `StateLock::lock` to obtain the state.
//!
//! Multi thread could call `StateLock::lock` at the same time. if the state is ready, the thread
//! would not block, else block until the state is ready.

mod state;
pub use state::State;

mod state_lock;
pub use state_lock::{StateGuard, StateLock};

#[cfg(test)]
mod tests {
    use super::*;
    use may::go;

    struct A;
    struct B;

    impl A {
        fn info(&self) {
            println!("A is ready");
        }
    }

    impl B {
        fn hello(&self) {
            println!("B is ready");
        }
    }

    impl State for A {
        fn tear_up() -> Self {
            A
        }
    }

    impl State for B {
        fn tear_up() -> Self {
            B
        }
    }

    #[test]
    fn test_waiter_map() {
        use std::sync::Arc;
        let state_lock = Arc::new(StateLock::new());
        let state_lock_1 = state_lock.clone();

        let state_a = state_lock.lock::<A>().unwrap();
        state_a.info();

        go!(move || {
            let state_b = state_lock_1.lock::<B>().unwrap();
            state_b.hello();
        });
    }
}
