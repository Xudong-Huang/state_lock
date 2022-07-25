use once_cell::sync::Lazy;

use crate::state::State;

use std::collections::BTreeMap;

// This is what we registered
pub struct StateRegistration {
    // state family name
    pub state_family: &'static str,
    // state name
    pub state: &'static str,
    // we are using lazy to avoid std service when register
    pub tear_up_fn: fn() -> Box<dyn State>,
}

inventory::collect!(StateRegistration);

type RegisteredState = BTreeMap<&'static str, fn() -> Box<dyn State>>;
type RegisteredStateSet = BTreeMap<&'static str, RegisteredState>;
// state registration
static REGISTERED_STATES: Lazy<RegisteredStateSet> = Lazy::new(|| {
    let mut map = BTreeMap::new();
    for registered in inventory::iter::<StateRegistration> {
        let state_map = map
            .entry(registered.state_family)
            .or_insert_with(BTreeMap::new);
        state_map
            .entry(registered.state)
            .or_insert(registered.tear_up_fn);
    }
    map
});

fn get_state_family(state_family: &str) -> &RegisteredState {
    REGISTERED_STATES
        .get(state_family)
        .expect("state set not found")
}

pub fn state_names(state_family: &str) -> impl Iterator<Item = &'static str> + '_ {
    let state_family = get_state_family(state_family);
    state_family.keys().copied()
}

pub fn tear_up_registered_state(state_family: &str, name: &str) -> Option<Box<dyn State>> {
    let state_family = get_state_family(state_family);
    let tear_up = state_family.get(&name)?;
    Some(tear_up())
}
