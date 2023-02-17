use may::go;
use state_lock::intertrait::cast::CastRef;
use state_lock::intertrait::cast_to;
use state_lock::{State, StateLock};

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

#[derive(State)]
#[family("StateIter")]
struct D;

trait Test {
    fn hello(&self);
}

#[cast_to]
impl Test for A {
    fn hello(&self) {
        println!("A is hello");
    }
}

#[cast_to]
impl Test for B {
    fn hello(&self) {
        println!("B is hello");
    }
}

#[cast_to]
impl Test for C {
    fn hello(&self) {
        println!("C is hello");
    }
}

#[cast_to]
impl Test for D {
    fn hello(&self) {
        println!("D is hello");
    }
}

fn make_test(state_name: &str) -> Box<dyn State> {
    match state_name {
        stringify!(A) => Box::new(A),
        stringify!(B) => Box::new(B),
        stringify!(C) => Box::new(C),
        stringify!(D) => Box::new(D),
        other => panic!("could create for state: {other}"),
    }
}

fn main() {
    env_logger::init();
    may::config().set_stack_size(6 * 1024);

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new_with_custom_tear_up(STATE_FAMILY, make_test));

    for _ in 0..100 {
        may::coroutine::scope(|scope| {
            for _ in 0..100 {
                state_lock.state_names().for_each(|name| {
                    let state_lock_clone = state_lock.clone();
                    go!(scope, move || {
                        let state = state_lock_clone.lock_by_state_name(name).unwrap();
                        println!("state: {state:?} waiting done");
                        assert_eq!(
                            Some(state.name()),
                            state_lock_clone.current_state().map(|s| s.name())
                        );
                        let test = state.as_dyn_state().cast::<dyn Test>().unwrap();
                        test.hello();
                    });
                });
            }
        });
        println!("==============================================================");
    }

    println!("states: {:?}", state_lock.state_names().collect::<Vec<_>>());
}
