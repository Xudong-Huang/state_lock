# state_lock

`StateLock` provide a sync primitive that can be used to wait for a state to be ready.

The state type must impl `State` trait, which contains the tear up and tear down logic
to prepare and destroy the state. They you can call the `StateLock::lock` to obtain the state.

Multi thread could call `StateLock::lock` at the same time. if the state is ready, the thread
would not block, else block until the state is ready.

## Usage
```rust
use may::go;
use state_lock::{RawState, State, StateLock};

const STATE_FAMILY: &str = "StateIter";

#[derive(State, Default)]
#[family(STATE_FAMILY)]
struct A;

#[derive(State, Default)]
#[family(STATE_FAMILY)]
struct B;

#[derive(State, Default)]
#[family("StateIter")]
struct C;

#[derive(State, Default)]
#[family("StateIter")]
struct D;

trait Test {
    fn hello(&self);
}

impl Test for A {
    fn hello(&self) {
        println!("A is hello");
    }
}

impl Test for B {
    fn hello(&self) {
        println!("B is hello");
    }
}

impl Test for C {
    fn hello(&self) {
        println!("C is hello");
    }
}

impl Test for D {
    fn hello(&self) {
        println!("D is hello");
    }
}

// we have to write this by hand, there is no way to automatically generate those code.
fn as_test<'a>(raw_state: &'a RawState) -> &'a dyn Test {
    match raw_state.name() {
        "A" => raw_state.as_state::<A>() as &dyn Test,
        "B" => raw_state.as_state::<B>() as &dyn Test,
        "C" => raw_state.as_state::<C>() as &dyn Test,
        "D" => raw_state.as_state::<D>() as &dyn Test,
        state_name => panic!("Unknown state: {}", state_name),
    }
}

fn main() {
    env_logger::init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new(STATE_FAMILY));

    for _ in 0..100 {
        may::coroutine::scope(|scope| {
            for _ in 0..100 {
                state_lock.state_names().for_each(|name| {
                    let state_lock_clone = state_lock.clone();
                    go!(scope, move || {
                        let state = state_lock_clone.lock_by_state_name(name).unwrap();
                        println!("state: {:?} waiting done", state);
                        let test = as_test(&state);
                        test.hello();
                    });
                });
            }
        });
        println!("==============================================================");
    }
}
```