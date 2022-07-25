use crate::registry::tear_up_registered_state;
use crate::StateLock;

use std::any::Any;
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

    fn family(&self) -> &'static str;

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
    pub(crate) fn new_from_name(state_lock: &StateLock, name: &str) -> Option<Self> {
        let state = Some(tear_up_registered_state(state_lock.state_family(), name)?);
        // it's safe to eliminate the life time here, basically they are equal
        unsafe { std::mem::transmute(StateWrapper { state_lock, state }) }
    }

    /// return the state name
    pub(crate) fn name(&self) -> &'static str {
        self.state.as_ref().unwrap().name()
    }

    /// return the state family name
    pub(crate) fn family(&self) -> &'static str {
        self.state.as_ref().unwrap().family()
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

        self.state_lock.wakeup_next_group();
    }
}

/// general state that can access the shared state
pub struct RawState<'a> {
    // we use `Arc` to track the state references
    // when all `StateWrapper`s are dropped, the state would be tear_down
    state: Arc<StateWrapper<'a>>,
}

unsafe impl<'a> Sync for RawState<'a> {}

impl<'a> Debug for RawState<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RawState{{ {}:{} }}",
            self.state.family(),
            self.state.name()
        )
    }
}

impl<'a> RawState<'a> {
    pub(crate) fn new(state: Arc<StateWrapper<'a>>) -> Self {
        RawState { state }
    }

    /// get the state name
    pub fn name(&self) -> &'static str {
        self.state.name()
    }

    /// get the state family name
    pub fn family(&self) -> &'static str {
        self.state.family()
    }

    /// convert to a concrete state type
    pub fn as_state<T: State>(&self) -> &T {
        self.state.downcast()
    }

    /// convert to StateGuard
    pub fn into_guard<T: State>(self) -> StateGuard<'a, T> {
        let _ = self.state.downcast::<T>(); // check type
        StateGuard {
            state: self.state,
            _phantom: PhantomData,
        }
    }
}

/// state guard that can access the shared state
pub struct StateGuard<'a, T: State> {
    // we use `Arc` to track the state references
    // when all `StateWrapper`s are dropped, the state would be tear_down
    state: Arc<StateWrapper<'a>>,
    _phantom: PhantomData<&'a T>,
}

unsafe impl<'a, T: State> Sync for StateGuard<'a, T> {}

impl<'a, T: State> Debug for StateGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RawState{{ {}:{} }}",
            self.state.family(),
            self.state.name()
        )
    }
}

impl<'a, T: State> StateGuard<'a, T> {
    /// get the state name
    pub fn name(&self) -> &'static str {
        self.state.name()
    }
    /// get the state family name
    pub fn family(&self) -> &'static str {
        self.state.family()
    }
}

impl<'a, T: State> Deref for StateGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.state.downcast()
    }
}
