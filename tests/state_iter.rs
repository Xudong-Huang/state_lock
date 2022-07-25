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

// we have to write this by hand, there is no way to automatically generate those code.
fn as_test<'a>(raw_state: &'a RawState) -> &'a dyn Test {
    match raw_state.name() {
        "A" => raw_state.as_state::<A>() as &dyn Test,
        "B" => raw_state.as_state::<B>() as &dyn Test,
        "C" => raw_state.as_state::<C>() as &dyn Test,
        state_name => panic!("Unknown state: {}", state_name),
    }
}

#[test]
fn test_state_lock() {
    // let _ = env_logger::builder()
    //     .filter_level(log::LevelFilter::Debug)
    //     .try_init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new(STATE_FAMILY));

    for _ in 0..1000 {
        may::coroutine::scope(|scope| {
            state_lock.state_names().for_each(|name| {
                let state_lock_clone = state_lock.clone();
                go!(scope, move || {
                    let state = state_lock_clone.lock_by_state_name(name).unwrap();
                    println!("state: {:?} waiting done", state);
                    let test = as_test(&state);
                    test.hello();
                });
            });
        });
    }
}
