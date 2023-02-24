use state_lock::intertrait::cast::CastRef;
use state_lock::intertrait::cast_to;
use state_lock::{State, StateLock};

const STATE_FAMILY: &str = "StateIter";

#[derive(State)]
#[family(STATE_FAMILY)]
struct A;

impl Default for A {
    fn default() -> Self {
        println!("create A");
        A
    }
}

impl Drop for A {
    fn drop(&mut self) {
        println!("drop A");
    }
}

#[derive(State, Default)]
#[family(STATE_FAMILY)]
struct B;

impl Drop for B {
    fn drop(&mut self) {
        println!("drop B");
    }
}

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

fn main() {
    env_logger::init();
    may::config().set_stack_size(6 * 1024);

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new(STATE_FAMILY));

    (0..3).for_each(|_| {
        let state = state_lock.lock_by_state_name(stringify!(A)).unwrap();
        let test = state.as_dyn_state().cast::<dyn Test>().unwrap();
        test.hello();
        // here state drop would cause create state frequently
    });

    let state = state_lock.lock_by_state_name(stringify!(B)).unwrap();
    let test = state.as_dyn_state().cast::<dyn Test>().unwrap();
    test.hello();
    println!("==============================================================");

    println!("states: {:?}", state_lock.state_names().collect::<Vec<_>>());
}
