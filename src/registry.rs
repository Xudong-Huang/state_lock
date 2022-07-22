use once_cell::sync::Lazy;

use crate::state::State;

use std::collections::BTreeMap;

// This is what we registered
pub struct StateRegistration {
    pub state: &'static str,
    // we are using lazy to avoid std service when register
    pub tear_up_fn: fn() -> Box<dyn State>,
}

inventory::collect!(StateRegistration);

pub fn tear_up_registered_state(id: &str) -> Box<dyn State> {
    type RegisteredState = BTreeMap<&'static str, fn() -> Box<dyn State>>;
    // state registration
    static REGISTERED_STATES: Lazy<RegisteredState> = Lazy::new(|| {
        let mut map = BTreeMap::new();
        for registered in inventory::iter::<StateRegistration> {
            map.entry(registered.state).or_insert(registered.tear_up_fn);
        }
        map
    });
    let tear_up = REGISTERED_STATES
        .get(&id)
        .expect("can't find state registration");
    tear_up()
}
