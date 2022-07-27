use indexmap::IndexMap;
use may::sync::Mutex;
use may_waiter::{TokenWaiter, ID};

use crate::state::{RawState, State, StateGuard, StateWrapper};

use std::fmt::{self, Debug};
use std::io;
use std::sync::{Arc, Weak};

struct StateLockInner {
    // waiter map, key is the state type id, value is the waiter
    map: IndexMap<String, Vec<ID>>,
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
    state_family: String,
}

impl Debug for StateLock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WaiterMap{{ ... }}")
    }
}

impl StateLock {
    /// crate a new state lock with the given state family name.
    /// it will panic if the state family is not registered.
    pub fn new(state_family: &str) -> Self {
        let count = crate::registry::state_names(state_family).count();
        StateLock {
            inner: Mutex::new(StateLockInner {
                map: IndexMap::with_capacity(count),
                state: None,
            }),
            state_family: state_family.into(),
        }
    }

    /// return the state family name
    pub fn state_family(&self) -> &str {
        &self.state_family
    }

    /// return all internal state names
    pub fn state_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        crate::registry::state_names(&self.state_family)
    }

    pub fn current_state(&self) -> Option<RawState> {
        self.inner
            .lock()
            .unwrap()
            .state
            .as_ref()
            .and_then(|s| s.upgrade())
            .map(RawState::new)
    }

    /// lock for a state by it's name
    /// since we can't get the state type, we have to return a state wrapper
    pub fn lock_by_state_name(&self, state_name: &str) -> io::Result<RawState> {
        if !self.state_names().any(|name| name == state_name) {
            let err_msg = format!("state {} is not registered", state_name);
            return Err(io::Error::new(io::ErrorKind::Other, err_msg));
        }

        let mut lock = self.inner.lock().unwrap();
        if let Some(s) = lock.state.as_ref().and_then(|s| s.upgrade()) {
            // if we are waiting for the same state, then just return
            if s.name() == state_name {
                trace!("{} state is already locked", s.name());
                return Ok(RawState::new(s));
            }

            // we have to wait until the state is setup
            let waiter = TokenWaiter::new();
            let waiters = lock
                .map
                .entry(state_name.to_string())
                .or_insert_with(Vec::new);

            // insert the waiter into the waiters queue
            let id = waiter.id().unwrap();
            trace!("{} state register a waiter {:?} ", state_name, id);
            waiters.push(id);
            // release the lock and let other thread to access the state lock
            drop(lock);
            // release the state ref before wait for the state to be setup
            // drop the state after release the lock, it may use the lock in sate drop
            drop(s);

            // wait for the state to be setup
            trace!("{} state is waiting for setup", state_name);
            let state = waiter.wait_rsp(None)?;
            trace!("{} state wait done", state_name);
            Ok(RawState::new(state))
        } else {
            // create a new state
            let state = Arc::new(StateWrapper::new_from_name(self, state_name).unwrap());
            lock.state = Some(Arc::downgrade(&state));
            let waiter_ids = lock.map.remove(state_name);
            drop(lock);

            trace!("{} state is set from empty", state_name);
            // wake up all waiters waiting for the same state
            if let Some(ids) = waiter_ids {
                for waiter_id in ids {
                    trace!(
                        "wakeup {} state, waiter {:?} (with same state)",
                        state_name,
                        waiter_id
                    );
                    TokenWaiter::set_rsp(waiter_id, state.clone());
                }
            }

            Ok(RawState::new(state))
        }
    }

    /// lock for a state by state concrete type
    pub fn lock<T: State>(&self) -> io::Result<StateGuard<T>> {
        let state_name = T::state_name();
        let state = self.lock_by_state_name(state_name)?;
        Ok(state.into_guard())
    }

    /// wait up all the waiters that are waiting for the state
    pub(crate) fn wakeup_next_group(&self) {
        let mut lock = self.inner.lock().unwrap();
        {
            // we already have live state, no need to wakeup
            if lock.state.as_ref().and_then(|s| s.upgrade()).is_some() {
                drop(lock);
                return;
            }
        }
        // have to wake up next group
        if let Some((new_state, waiters)) = lock.map.shift_remove_index(0) {
            trace!("wakeup_next_group to state {}", new_state);
            // create a new state from the id
            let state = StateWrapper::new_from_name(self, &new_state);
            let state = Arc::new(state.expect("state name not found"));

            // need first drop the old state
            lock.state.replace(Arc::downgrade(&state));
            trace!("{} state is set from last state", state.name());

            // must first drop the lock, then wakeup the waiters
            drop(lock);
            // wait up all the waiters that are waiting for the state
            for waiter_id in waiters {
                trace!("wakeup {} state, waiter {:?}", new_state, waiter_id);
                TokenWaiter::set_rsp(waiter_id, state.clone());
            }
        } else {
            trace!("state cleared!!!!");
            lock.state = None
        }
    }
}
