use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub route: Addr,
    pub tokens: BTreeMap<String, Token>,
    pub handled_tickets: BTreeSet<String>,
    pub handled_directives: BTreeSet<u64>,
    pub target_chain_factor: BTreeMap<String, u128>,
    pub fee_token: Option<String>,
    pub fee_token_factor: Option<u128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Token {
    pub name: String,
    pub denom: String,
    pub settlement_chain: String,
}

pub const STATE: Item<State> = Item::new("state");

pub fn read_state<F, R>(store: &dyn Storage, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    f(&STATE.load(store).expect("State not initialized!"))
}
