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
    fn name(&self) -> &str;

    /// tear up the state, just create the state
    fn tear_up() -> Self
    where
        Self: Sized;

		/// tear down the state, just drop the state
    fn tear_down(&mut self) {
		println!("{} state tear down", self.name());
	}

	/// get the state as any type
	fn as_any(&self) -> &dyn Any;
}

/// internal state wrapper that would call tear_down automatically when dropped
pub(crate) struct StateWrapper<'a> {
    // State lock hold the state, it's safe to have the reference
    state_lock: &'a StateLock,
    state: Box<dyn State>,
}

impl<'a> StateWrapper<'a> {
    pub fn new<T: State>(state_lock: &StateLock) -> Self {
        let state = Box::new(T::tear_up());
        // it's safe to eliminate the life time here, basically they are equal
        unsafe { std::mem::transmute(StateWrapper { state_lock, state }) }
    }

    pub fn new_from_id(state_lock: &StateLock, id: &str) -> Self {
        let state = tear_up_registered_state(id);
        // it's safe to eliminate the life time here, basically they are equal
        unsafe { std::mem::transmute(StateWrapper { state_lock, state }) }
    }

    pub fn type_id(&self) -> TypeId {
        // self.state.type_id()
        self.state.as_ref().type_id()
    }

    pub fn name(&self) -> &str {
        self.state.name()
    }

    /// downcast to a concrete state type
    pub fn as_state<T: State>(&self) -> &T {
        let any = self.state.as_any();
        any.downcast_ref::<T>().expect("wrong state cast")
    }
}

impl<'a> Drop for StateWrapper<'a> {
    fn drop(&mut self) {
        self.state_lock.wakeup_next_group(self);
        self.state.tear_down();
        println!("{} state is dropped", self.name());
    }
}
