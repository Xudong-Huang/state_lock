use may::go;
use state_lock::{State, StateLock};

#[derive(State, Default)]
struct A;

#[derive(State, Default)]
struct B;

#[derive(State, Default)]
struct C;

#[test]
fn test_state_lock() {
    // let _ = env_logger::builder()
    //     .filter_level(log::LevelFilter::Debug)
    //     .try_init();

    use std::sync::Arc;
    let state_lock = Arc::new(StateLock::new());

    for _ in 0..1000 {
        may::coroutine::scope(|scope| {
            state_lock.state_names().for_each(|name| {
                let state_lock_clone = state_lock.clone();
                go!(scope, move || {
                    let state = state_lock_clone.lock_by_state_name(name).unwrap();
                    println!("state name: {} waiting done", state.name());
                });
            });
        });
    }
}
