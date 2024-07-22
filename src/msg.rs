use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::Token;

#[cw_serde]
pub struct InstantiateMsg {
    pub route: Addr,
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
    },
    RedeemToken {
        token_id: String,
        receiver: String,
        amount: String,
    },
    MintRunes {
        token_id: String,
        receiver: Addr,
    },
    BurnToken {
        token_id: String,
        amount: String,
    },
}

#[cw_serde]
pub enum Directive {
    AddToken {
        settlement_chain: String,
        token_id: String,
        name: String,
    },
    UpdateFee {
        factor: Factor,
    },
}

#[cw_serde]
pub enum Factor {
    FeeTokenFactor {
        fee_token: String,
        fee_token_factor: u128,
    },
    TargetChainFactor {
        target_chain_id: String,
        target_chain_factor: u128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetTokenResponse)]
    GetTokenList,
    #[returns(GetFeeResponse)]
    GetFeeInfo,
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
