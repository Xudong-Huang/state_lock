use crate::StateLock;

use std::any::{Any, TypeId};

/// any type that impl `State` can be used by `StateLock`
/// a `state` is essentially a type that can be crated by `StateLock`
/// any task that try to lock the state would block until the state is ready
/// the task then could get a reference to the state
pub trait State: Any + Send + Sync {
    /// tear up the state, just create the state
    fn tear_up() -> Self
    where
        Self: Sized;
    /// tear down the state, just drop the state
    fn tear_down(&mut self) {}
}

/// internal state wrapper that would call tear_down automatically when dropped
pub(crate) struct StateWrapper<'a> {
    // State lock hold the state, it's safe to have the reference
    sate_lock: &'a StateLock,
    state: Box<dyn State>,
}

impl<'a> StateWrapper<'a> {
    pub fn new<T: State>(sate_lock: &StateLock) -> Self {
        let state = Box::new(T::tear_up());
        // it's safe to eliminate the life time here
        unsafe { std::mem::transmute(StateWrapper { sate_lock, state }) }
    }

    pub fn new_from_id(state_lock: &StateLock, id: TypeId) -> Self {
        let _ = state_lock;
        let _ = id;
        todo!()
    }

    pub fn type_id(&self) -> TypeId {
        self.state.type_id()
    }

    pub fn as_state<T: State>(&self) -> &T {
        let any = &self.state as &dyn Any;
        any.downcast_ref::<T>().expect("wrong state cast")
    }
}

impl<'a> Drop for StateWrapper<'a> {
    fn drop(&mut self) {
        self.sate_lock.wakeup_next_group();
        self.state.tear_down();
    }
}
