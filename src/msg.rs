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
    TestMsg{
        text: String,
    },
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
    MintRunes {
        token_id: String,
        receiver: Addr,
        target_chain: String,
    },
    BurnToken {
        token_id: String,
        amount: String,
        target_chain: String,
    },
    UpdateRoute {
        route: Addr,
    }
}


// #[cw_serde]
// pub enum Directive {
//     AddChain {
//         chain: Chain,
//     },
//     AddToken {
//         settlement_chain: String,
//         token_id: String,
//         name: String,
//     },
//     UpdateChain {
//         chain: Chain,
//     },
//     UpdateToken {

//     },
//     ToggleChainState {
//         chain_id: String,
//         action: ToggleAction
//     },
//     UpdateFee {
//         factor: Factor,
//     },
// }

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
