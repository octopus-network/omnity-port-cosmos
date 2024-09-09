use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::route::{Directive, Token};

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
    },
    RedeemToken {
        token_id: String,
        receiver: String,
        amount: String,
        target_chain: String,
    },
    UpdateRoute {
        route: Addr,
    },
    RedeemSetting {
        token_id: String,
        target_chain: String,
        min_amount: String,
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
