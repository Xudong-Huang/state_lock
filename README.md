# state_lock

`StateLock` provide a sync primitive that can be used to wait for a state to be ready.

The state type must impl `State` trait, which contains the tear up and tear down logic
to prepare and destroy the state. They you can call the `StateLock::lock` to obtain the state.

Multi thread could call `StateLock::lock` at the same time. if the state is ready, the thread
would not block, else block until the state is ready.

## Usage
```rust
use may::go;
use state_lock::{State, StateLock, StateRegistration};
use std::any::Any;

#[derive(State, Default)]
struct A;
impl A {
    fn info(&self) {
        println!("A info");
    }
}

#[derive(State, Default)]
struct B;
impl B {
    fn hello(&self) {
        println!("B is hello");
    }
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new());
    let state_lock_1 = state_lock.clone();
    let state_lock_2 = state_lock.clone();

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
```

## Output
```
wait for A
[2022-07-22T11:17:50Z DEBUG state_lock::lock] B state is set from empty
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state register a waiter ID(6848988010368)
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state is waiting for setup
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state register a waiter ID(18990301453184)
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state is waiting for setup
B is hello
[2022-07-22T11:17:50Z DEBUG state_lock::lock] B state is already locked
B is hello
[2022-07-22T11:17:50Z DEBUG state_lock::state] B state tear down
[2022-07-22T11:17:50Z DEBUG state_lock::state] B state is dropped
[2022-07-22T11:17:50Z DEBUG state_lock::lock] wakeup_next_group for state A
[2022-07-22T11:17:50Z DEBUG state_lock::lock] wakeup A state, waiter ID(6848988010368)
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state wait done
wait for A done
[2022-07-22T11:17:50Z DEBUG state_lock::lock] wakeup A state, waiter ID(18990301453184)
A info
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state is set from B state
[2022-07-22T11:17:50Z DEBUG state_lock::lock] A state wait done
A info
[2022-07-22T11:17:50Z DEBUG state_lock::state] A state tear down
[2022-07-22T11:17:50Z DEBUG state_lock::state] A state is dropped
[2022-07-22T11:17:50Z DEBUG state_lock::lock] state cleared!!!!
```