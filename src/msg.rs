use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub chain_key: Vec<u8>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ExecDirective {
        directive: Directive,
        signature: Vec<u8>,
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
}

#[cw_serde]
pub enum Directive {
    AddToken { token_id: String, name: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(GetCountResponse)]
    GetCount {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetCountResponse {
    pub count: i32,
}
