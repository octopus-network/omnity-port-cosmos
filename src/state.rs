use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::Item;

use crate::route::{Chain, ChainId, ChainState, Token};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub route: Addr,
    pub admin: Addr,
    pub tokens: BTreeMap<String, Token>,
    pub handled_tickets: BTreeSet<String>,
    pub handled_directives: BTreeSet<u64>,
    pub target_chain_factor: BTreeMap<String, u128>,
    pub fee_token: Option<String>,
    pub fee_token_factor: Option<u128>,
    pub counterparties: BTreeMap<ChainId, Chain>,
    pub chain_id: ChainId,
    pub chain_state: ChainState
}

pub const STATE: Item<State> = Item::new("state");

pub fn read_state<F, R>(store: &dyn Storage, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    f(&STATE.load(store).expect("State not initialized!"))
}