use std::collections::{BTreeMap, BTreeSet, HashMap};

use cosmwasm_schema::cw_serde;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage};

use crate::route::{Chain, ChainId, ChainState, Token, TokenId};
use cw_storage_plus::{Map, Item};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub route: Addr,
    pub admin: Addr,
    pub tokens: BTreeMap<TokenId, Token>,
    pub handled_tickets: BTreeSet<String>,
    pub handled_directives: BTreeSet<u64>,
    pub target_chain_factor: BTreeMap<ChainId, u128>,
    pub fee_token: Option<String>,
    pub fee_token_factor: Option<u128>,
    pub counterparties: BTreeMap<ChainId, Chain>,
    pub chain_id: ChainId,
    pub chain_state: ChainState,
    #[serde(default)]
    pub target_chain_redeem_min_amount: BTreeMap<(TokenId, ChainId), String>,
    #[serde(default)]
    pub generate_ticket_sequence: u64,
    #[serde(default)]
    pub ckbtc_token_id: String, 
    #[serde(default)]
    pub allbtc_token_denom: String, 
    #[serde(default)]
    pub allbtc_swap_pool_id: u64, 
    // key is replaced id, value is original id
    #[serde(default)]
    pub runes_replaced_id_map: HashMap<String, String>
}

impl State {
    pub fn replace_token_id_if_runes(&mut self, token_id: &str) -> String {
        if let Some(runes_token_id) = self.runes_replaced_id_map.get(token_id) {
            runes_token_id.to_string()
        } else {
            token_id.to_string()
        }
    }
}

pub const STATE: Item<State> = Item::new("state");
pub const GENERATE_TICKET_REQ: Map<u64, GenerateTicketReq> = Map::new("generate-ticket-req");

pub fn read_state<F, R>(store: &dyn Storage, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    f(&STATE.load(store).expect("State not initialized!"))
}

#[cw_serde]
pub struct GenerateTicketReq {
    pub seq: u64,
    pub target_chain_id: String,
    pub sender: String,
    pub receiver: String,
    pub token_id: String,
    pub amount: String,
    pub action: TxAction,
    pub timestamp: u64,
    pub block_height: u64,
    pub memo: Option<String>,
    pub fee_token: String,
    pub fee_amount: String,
}

#[cw_serde]
pub enum TxAction {
    Transfer,
    Redeem,
    Burn,
    RedeemIcpChainKeyAssets(IcpChainKeyToken)
}

#[cw_serde]
pub enum IcpChainKeyToken {
    CKBTC
}