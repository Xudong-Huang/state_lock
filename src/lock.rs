use indexmap::IndexMap;
use may::sync::Mutex;
use may_waiter::{TokenWaiter, ID};

use crate::state::{RawState, State, StateGuard, StateWrapper};

use std::fmt::{self, Debug};
use std::io;
use std::sync::{Arc, Weak};

struct StateLockInner {
    // waiter map, key is the state type id, value is the waiter
    map: IndexMap<&'static str, Vec<ID>>,
    // track the current state, static life time for self ref
    state: Option<Weak<StateWrapper<'static>>>,
}

unsafe impl Send for StateLockInner {}

/// `StateLock` that could be used to lock for a state.
///
/// After call `StateLock::lock` a `StateGuard` would be returned,
/// then you could use the `StateGuard` to access the state.
pub struct StateLock {
    inner: Mutex<StateLockInner>,
}

impl Debug for StateLock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WaiterMap{{ ... }}")
    }
}

impl Default for StateLock {
    fn default() -> Self {
        StateLock::new()
    }
}

impl StateLock {
    /// crate a new state lock
    /// TODO: how to distinguish two different types state lock?
    pub fn new() -> Self {
        StateLock {
            inner: Mutex::new(StateLockInner {
                map: IndexMap::new(),
                state: None,
            }),
        }
    }

    /// return all internal state names
    pub fn state_names(&self) -> impl Iterator<Item = &'static str> {
        crate::registry::state_names()
    }

    /// lock for a state by it's name
    /// since we can't get the state type, we have to return a state wrapper
    pub fn lock_by_state_name(&self, state_name: &'static str) -> io::Result<RawState> {
        if !self.state_names().any(|name| name == state_name) {
            let err_msg = format!("state {} is not registered", state_name);
            return Err(io::Error::new(io::ErrorKind::Other, err_msg));
        }

        let mut lock = self.inner.lock().unwrap();
        let state = lock.state.as_ref().and_then(|s| s.upgrade());
        if let Some(s) = state {
            // if we are waiting for the same state, then just return
            if s.name() == state_name {
                debug!("{} state is already locked", s.name());
                return Ok(RawState::new(self, s));
            }

            // we have to wait until the state is setup
            let waiter = TokenWaiter::new();
            let waiters = lock.map.entry(state_name).or_insert_with(Vec::new);

            // insert the waiter into the waiters queue
            let id = waiter.id().unwrap();
            debug!("{} state register a waiter {:?} ", state_name, id);
            waiters.push(id);
            // release the lock and let other thread to access the state lock
            drop(lock);
            // release the state ref before wait for the state to be setup
            // drop the state after release the lock, it may use the lock in sate drop
            drop(s);

            // wait for the state to be setup
            debug!("{} state is waiting for setup", state_name);
            let state = waiter.wait_rsp(None)?;
            debug!("{} state waite done", state_name);
            Ok(RawState::new(self, state))
        } else {
            // the last state is just released, check there is no same state waiter
            assert!(lock.map.get(state_name).is_none());
            // create a new state
            let state = Arc::new(StateWrapper::new_from_name(self, state_name).unwrap());
            lock.state = Some(Arc::downgrade(&state));
            debug!("{} state is set from empty", state_name);
            Ok(RawState::new(self, state))
        }
    }

    /// lock for a state by state concrete type
    pub fn lock<T: State>(&self) -> io::Result<StateGuard<T>> {
        let state_name = T::state_name();
        let state = self.lock_by_state_name(state_name)?;
        Ok(state.into_guard())
    }

    /// wait up all the waiters that are waiting for the state
    pub(crate) fn wakeup_next_group(&self, old_state: &str) {
        let mut lock = self.inner.lock().unwrap();
        if let Some((new_state, waiters)) = lock.map.shift_remove_index(0) {
            debug!("wakeup_next_group for state {}", new_state);
            // create a new state from the id
            let state = StateWrapper::new_from_name(self, new_state).expect("state name not found");
            let state = Arc::new(state);
            // wait up all the waiters that are waiting for the state
            for waiter_id in waiters {
                debug!("wakeup {} state, waiter {:?}", new_state, waiter_id);
                TokenWaiter::set_rsp(waiter_id, state.clone());
            }
            // need first drop the old state
            lock.state.replace(Arc::downgrade(&state));
            debug!("{} state is set from {} state", state.name(), old_state);
        } else {
            debug!("state cleared!!!!");
            lock.state = None
        }
    }
}
