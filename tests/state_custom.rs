use may::go;
use state_lock::{RawState, State, StateLock};
// use state_lock::default::DefaultImplement;

const STATE_FAMILY: &str = "StateIter";

#[derive(State)]
#[family(STATE_FAMILY)]
struct A;

#[derive(State)]
#[family(STATE_FAMILY)]
struct B;

#[derive(State)]
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
        stringify!(A) => raw_state.as_state::<A>() as &dyn Test,
        stringify!(B) => raw_state.as_state::<B>() as &dyn Test,
        stringify!(C) => raw_state.as_state::<C>() as &dyn Test,
        state_name => panic!("Unknown state: {}", state_name),
    }
}

fn make_test(state_name: &str) -> Box<dyn State> {
    match state_name {
        stringify!(A) => Box::new(A),
        stringify!(B) => Box::new(B),
        stringify!(C) => Box::new(C),
        other => panic!("could create for state: {}", other),
    }
}

#[test]
fn test_state_custom_iter() {
    env_logger::init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new_with_custom_tear_up(STATE_FAMILY, make_test));

    for _ in 0..1000 {
        may::coroutine::scope(|scope| {
            state_lock.state_names().for_each(|name| {
                let state_lock_clone = state_lock.clone();
                go!(scope, move || {
                    let state = state_lock_clone.lock_by_state_name(name).unwrap();
                    println!("state: {:?} waiting done", state);
                    assert_eq!(
                        Some(state.name()),
                        state_lock_clone.current_state().map(|s| s.name())
                    );
                    let test = as_test(&state);
                    test.hello();
                });
            });
        });
    }
}
