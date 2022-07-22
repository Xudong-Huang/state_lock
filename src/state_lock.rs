use indexmap::IndexMap;
use may_waiter::{TokenWaiter, ID};

use crate::state::{State, StateWrapper};

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::io;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};
// use std::time::Duration;
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
    map: IndexMap<TypeId, Vec<ID>>,
    // track the current state, static life time for self ref
    state: Option<Weak<StateWrapper<'static>>>,
}

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
        let waiter = if let Some(s) = state {
            let state_type = TypeId::of::<T>();
            // if we are waiting for the same state, then just return
            if s.type_id() == state_type {
                return Ok(StateGuard::new(self, s.clone()));
            }
            // we have to wait until the state is setup
            let waiter = TokenWaiter::new();
            let waiters = lock.map.entry(state_type).or_insert_with(Vec::new);
            // insert the waiter into the waiters queue
            waiters.push(waiter.id().unwrap());
            waiter
        } else {
            // the last state is just released, check there is no same state waiter
            assert!(lock.map.get(&TypeId::of::<T>()).is_none());
            // create a new state
            let state = Arc::new(StateWrapper::new::<T>(self));
            lock.state = Some(Arc::downgrade(&state));
            return Ok(StateGuard::new(self, state));
        };
        // release the lock and let other thread to access the state lock
        drop(lock);
        // wait for the state to be setup
        let state = waiter.wait_rsp(None)?;
        // assert_eq!(state.type_id(), TypeId::of::<T>());
        Ok(StateGuard::new(self, state))
    }

    /// wait up all the waiters that are waiting for the state
    pub(crate) fn wakeup_next_group(&self) {
        let mut lock = self.inner.lock().unwrap();
        if let Some((id, waiters)) = lock.map.shift_remove_index(0) {
            // create a new state from the id
            let state = Arc::new(StateWrapper::new_from_id(self, id));
            // wait up all the waiters that are waiting for the state
            for waiter_id in waiters {
                TokenWaiter::set_rsp(waiter_id, state.clone());
            }
            lock.state = Some(Arc::downgrade(&state));
        } else {
            lock.state = None
        }
    }
}
