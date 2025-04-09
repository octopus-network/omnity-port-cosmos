use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::{route::{Directive, Token}, state::{State, TxAction}};

pub mod reply_msg_id {
    pub const REDEEM_REPLY_ID: u64 = 1;
    pub const GENERATE_TICKET_REPLY_ID: u64 = 2;
    pub const MINT_TOKEN_REPLY_ID: u64 = 3;
    pub const SWAP_CKBTC_TO_ALLBTC_REPLY_ID: u64 = 4;
    pub const SWAP_ALLBTC_TO_CKBTC_REPLY_ID: u64 = 5;
    pub const SEND_ALLBTC_REPLY_ID: u64 = 6;
}

#[cw_serde]
pub struct InstantiateMsg {
    pub route: Addr,
    pub chain_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    ExecDirective {
        seq: u64,
        directive: Directive,
    },
    PrivilegeMintToken {
        ticket_id: String,
        token_id: String,
        receiver: Addr,
        amount: String,
        // transmuter token into another token then send to user
        transmuter: Option<String>
    },
    RedeemToken {
        token_id: String,
        receiver: String,
        amount: String,
        target_chain: String,
    },
    RedeemAllBTC {
        receiver: String,
        amount: String,
        target_chain: String,
    },
    GenerateTicket {
        token_id: String,
        sender: String,
        receiver: String,
        amount: String,
        target_chain: String,
        action: TxAction,
        memo: Option<String>,
    },
    UpdateRoute {
        route: Addr,
    },
    RedeemSetting {
        token_id: String,
        target_chain: String,
        min_amount: String,
    },
    UpdateToken {
        token_id: String,
        name: String,
        symbol: String,
        decimals: u8,
        icon: Option<String>,
    },
    RefundToken {
        denom: String,
        receiver: String,
        amount: String,
    }
}

#[cw_serde]
pub enum ToggleAction {
    Activate,
    Deactivate,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(State)]
    GetState {},
    #[returns(GetTokenResponse)]
    GetTokenList {},
    #[returns(GetFeeResponse)]
    GetFeeInfo {},
    #[returns(GetTargetChainFeeResponse)]
    GetTargetChainFee {
        target_chain: String,
    },
}

#[cw_serde]
pub struct GetTargetChainFeeResponse {
    pub target_chain: String,
    pub fee_token: Option<String>,
    pub fee_token_factor: Option<u128>,
    pub fee_amount: Option<u128>,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetTokenResponse {
    pub tokens: Vec<Token>,
}

#[cw_serde]
pub struct GetFeeResponse {
    pub fee_token: Option<String>,
    pub fee_token_factor: Option<u128>,
    pub target_chain_factor: BTreeMap<String, u128>,
}
