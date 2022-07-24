use may::go;
use state_lock::{State, StateLock, StateRegistration};
use std::any::Any;

struct A;

impl A {
    fn make() -> Box<dyn State> {
        Box::new(A)
    }

    fn info(&self) {
        println!("A info");
    }
}

impl State for A {
    fn state_name() -> &'static str {
        stringify!(A)
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

state_lock::inventory::submit! {
    StateRegistration {
        state_set: "A set",
        state: stringify!(A),
        tear_up_fn: A::make,
    }
}

#[derive(State, Default)]
struct B;
impl B {
    fn hello(&self) {
        println!("B is hello");
    }
}

#[test]
fn test_state_lock() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new("A set"));
    let state_lock_1 = state_lock.clone();
    let state_lock_2 = state_lock.clone();

    state_lock.state_names().for_each(|name| {
        println!("state name: {}", name);
    });

    go!(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let state_a1 = state_lock_2.lock::<A>().unwrap();
        state_a1.info();
    });

    go!(move || {
        let state_b = state_lock_1.lock::<B>().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        state_b.hello();
        let state_b1 = state_lock_1.lock::<B>().unwrap();
        state_b1.hello();
    });

    println!("wait for A");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let state_a = state_lock.lock::<A>().unwrap();
    println!("wait for A done");
    state_a.info();
}
