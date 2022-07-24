use crate::registry::tear_up_registered_state;
use crate::StateLock;

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

/// any type that impl `State` can be used by `StateLock`
/// a `state` is essentially a type that can be crated by `StateLock`
/// any task that try to lock the state would block until the state is ready
/// the task then could get a reference to the state
pub trait State: Any + Sync {
    /// unique state name, just the type name
    fn state_name() -> &'static str
    where
        Self: Sized;

    /// unique state name, just the type name
    fn name(&self) -> &'static str;

    /// tear up the state, just create the state
    fn tear_up() -> Self
    where
        Self: Sized;

    /// tear down the state, just drop the state
    fn tear_down(&mut self) {
        debug!("{} state tear down", self.name());
    }

    /// get the state as any type
    fn as_any(&self) -> &dyn Any;
}

/// internal state wrapper that would call tear_down automatically when dropped
pub(crate) struct StateWrapper<'a> {
    // State lock hold the state, it's safe to have the reference
    state_lock: &'a StateLock,
    // State is `Sync` but not `Send`
    state: Option<Box<dyn State>>,
}

impl<'a> StateWrapper<'a> {
    pub(crate) fn new<T: State>(state_lock: &StateLock) -> Self {
        let state = Some(Box::new(T::tear_up()) as Box<dyn State>);
        // it's safe to eliminate the life time here, basically they are equal
        unsafe { std::mem::transmute(StateWrapper { state_lock, state }) }
    }

    pub(crate) fn new_from_name(state_lock: &StateLock, name: &str) -> Option<Self> {
        let state = Some(tear_up_registered_state(name)?);
        // it's safe to eliminate the life time here, basically they are equal
        unsafe { std::mem::transmute(StateWrapper { state_lock, state }) }
    }

    /// return the state type id
    pub(crate) fn state_type_id(&self) -> TypeId {
        self.state.as_ref().unwrap().as_ref().type_id()
    }

    /// return the state name
    pub(crate) fn name(&self) -> &str {
        self.state.as_ref().unwrap().name()
    }

    /// downcast to a concrete state type
    pub(crate) fn downcast<T: State>(&self) -> &T {
        let any = self.state.as_ref().unwrap().as_any();
        any.downcast_ref::<T>().expect("wrong state cast")
    }
}

impl<'a> Drop for StateWrapper<'a> {
    fn drop(&mut self) {
        let mut state = self.state.take().unwrap();
        state.tear_down();
        let old_state = state.name();
        // we should drop the old state completely before setup the new state
        drop(state);
        debug!("{} state is dropped", old_state);

        self.state_lock.wakeup_next_group(old_state);
    }
}

/// general state that can access the shared state
pub struct RawState<'a> {
    _lock: &'a StateLock,
    // we use `Arc` to track the state references
    // when all `StateWrapper`s are dropped, the state would be tear_down
    state: Arc<StateWrapper<'a>>,
}

// unsafe impl<'a> Sync for RawState<'a> {}

impl<'a> Debug for RawState<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawState{{ ... }}")
    }
}

impl<'a> RawState<'a> {
    pub(crate) fn new(lock: &'a StateLock, state: Arc<StateWrapper<'a>>) -> Self {
        RawState { state, _lock: lock }
    }

    /// get the state name
    pub fn name(&self) -> &str {
        self.state.name()
    }

    /// downcast to a concrete state type
    pub fn downcast<T: State>(&self) -> &T {
        self.state.downcast::<T>()
    }
}

/// state guard that can access the shared state
pub struct StateGuard<'a, T: State> {
    _lock: &'a StateLock,
    // we use `Arc` to track the state references
    // when all `StateWrapper`s are dropped, the state would be tear_down
    state: Arc<StateWrapper<'a>>,
    _phantom: PhantomData<T>,
}

unsafe impl<'a, T: State> Sync for StateGuard<'a, T> {}

impl<'a, T: State> Debug for StateGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StateGuard{{ ... }}")
    }
}

impl<'a, T: State> StateGuard<'a, T> {
    pub(crate) fn new(lock: &'a StateLock, state: Arc<StateWrapper<'a>>) -> Self {
        assert_eq!(state.state_type_id(), TypeId::of::<T>());
        StateGuard {
            state,
            _lock: lock,
            _phantom: PhantomData,
        }
    }

    /// get the state name
    pub fn name(&self) -> &str {
        self.state.name()
    }
}

impl<'a, T: State> Deref for StateGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.state.downcast()
    }
}
