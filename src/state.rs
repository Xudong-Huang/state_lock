use crate::registry::tear_up_registered_state;
use crate::StateLock;

use std::any::{Any, TypeId};

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
/// TODO: better name and documentation
pub struct StateWrapper<'a> {
    // State lock hold the state, it's safe to have the reference
    state_lock: &'a StateLock,
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
    pub fn state_type_id(&self) -> TypeId {
        self.state.as_ref().unwrap().as_ref().type_id()
    }

    /// return the state name
    pub fn name(&self) -> &str {
        self.state.as_ref().unwrap().name()
    }

    /// downcast to a concrete state type
    pub fn as_state<T: State>(&self) -> &T {
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
