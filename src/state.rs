use crate::StateLock;

use std::any::{Any, TypeId};
use std::sync::Arc;

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
pub(crate) struct StateWrapper {
    sate_lock: Arc<StateLock>,
    state: Box<dyn State>,
}

impl StateWrapper {
    pub fn new<T: State>(sate_lock: Arc<StateLock>) -> Self {
        let state = Box::new(T::tear_up());
        StateWrapper { sate_lock, state }
    }

    pub fn new_from_id(sate_lock: Arc<StateLock>, id: TypeId) -> Self {
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

impl Drop for StateWrapper {
    fn drop(&mut self) {
        self.sate_lock.wakeup_next_group();
        self.state.tear_down();
    }
}
