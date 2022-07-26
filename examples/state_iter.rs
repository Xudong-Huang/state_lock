use intertrait::cast::CastRef;
use intertrait::cast_to;
use may::go;
use state_lock::{State, StateLock};

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

// we have to write this by hand, there is no way to automatically generate those code.
// you can use `state_names` to get all the states
// `println!("states: {:?}", state_lock.state_names().collect::<Vec<_>>());`
// macro_rules! as_dyn {
//     ($raw_state:expr, $dst:ident) => {
//         match $raw_state.name() {
//             stringify!(A) => $raw_state.as_state::<A>() as &dyn $dst,
//             stringify!(B) => $raw_state.as_state::<B>() as &dyn $dst,
//             stringify!(C) => $raw_state.as_state::<C>() as &dyn $dst,
//             stringify!(D) => $raw_state.as_state::<D>() as &dyn $dst,
//             state_name => panic!("Unknown state: {}", state_name),
//         }
//     };
// }

fn main() {
    env_logger::init();
    may::config().set_stack_size(6 * 1024);

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
                        assert_eq!(
                            Some(state.name()),
                            state_lock_clone.current_state().map(|s| s.name())
                        );
                        // let test = as_dyn!(state, Test);
                        let test = state.as_dyn().cast::<dyn Test>().unwrap();
                        test.hello();
                    });
                });
            }
        });
        println!("==============================================================");
    }

    println!("states: {:?}", state_lock.state_names().collect::<Vec<_>>());
}
