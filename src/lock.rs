use indexmap::IndexMap;
use may_waiter::{TokenWaiter, ID};

use crate::state::{State, StateWrapper};

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::io;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};
use std::marker::PhantomData;

/// state guard that can access the shared state
// #[derive(Debug)]
pub struct StateGuard<'a, T: State> {
    _lock: &'a StateLock,
    // we use `Arc` to track the state references
    // when all `StateGuard`s are dropped, the state would be tear_down
    state: Arc<StateWrapper<'a>>,
    // we use *mut T to prevent send
    _phantom: PhantomData<*mut T>,
}

unsafe impl<'a, T: State> Sync for StateGuard<'a, T> {}

impl<'a, T: State> StateGuard<'a, T> {
    fn new(lock: &'a StateLock, state: Arc<StateWrapper<'a>>) -> Self {
        assert_eq!(state.type_id(), TypeId::of::<T>());
        StateGuard {
            state,
            _lock: lock,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: State> Deref for StateGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.state.as_state()
    }
}

struct StateLockInner {
    // waiter map, key is the state type id, value is the waiter
    map: IndexMap<&'static str, Vec<ID>>,
    // track the current state, static life time for self ref
    state: Option<Weak<StateWrapper<'static>>>,
}

unsafe impl Send for StateLockInner {}

/// `StateLock` that could be used to wait response for a state
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
    pub fn new() -> Self {
        StateLock {
            inner: Mutex::new(StateLockInner {
                map: IndexMap::new(),
                state: None,
            }),
        }
    }

    pub fn lock<T: State>(&self) -> io::Result<StateGuard<T>> {
        let mut lock = self.inner.lock().unwrap();
        let state = lock.state.as_ref().and_then(|s| s.upgrade());
        if let Some(s) = state {
            let state_type = TypeId::of::<T>();
            // if we are waiting for the same state, then just return
            if s.type_id() == state_type {
                debug!("{} state is already locked", s.name());
                return Ok(StateGuard::new(self, s.clone()));
            } else {
                // release the state ref before wait for the state to be setup
                drop(s);
            }

            // we have to wait until the state is setup
            let waiter = TokenWaiter::new();
            let waiters = lock.map.entry(T::state_name()).or_insert_with(Vec::new);
            // insert the waiter into the waiters queue
            let id = waiter.id().unwrap();
            debug!("{} state register a waiter {:?} ", T::state_name(), id);
            waiters.push(id);
            // release the lock and let other thread to access the state lock
            drop(lock);

            // wait for the state to be setup
            debug!("{} state is waiting for setup", T::state_name());
            let state = waiter.wait_rsp(None)?;
            debug!("{} state waite done", T::state_name());
            Ok(StateGuard::new(self, state))
        } else {
            // the last state is just released, check there is no same state waiter
            assert!(lock.map.get(T::state_name()).is_none());
            // create a new state
            let state = Arc::new(StateWrapper::new::<T>(self));
            lock.state = Some(Arc::downgrade(&state));
            debug!("{} state is set from empty", T::state_name());
            Ok(StateGuard::new(self, state))
        }
    }

    /// wait up all the waiters that are waiting for the state
    pub(crate) fn wakeup_next_group(&self, old_state: &str) {
        let mut lock = self.inner.lock().unwrap();
        if let Some((new_state, waiters)) = lock.map.shift_remove_index(0) {
            debug!("wakeup_next_group for state {}", new_state);
            // create a new state from the id
            let state = Arc::new(StateWrapper::new_from_id(self, new_state));
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
