use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct MintTokenPayload {
    pub ticket_id: String,
    pub token_id: String,
    pub receiver: Addr,
    pub amount: String,
    pub transmuter: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemAllBTC {
    pub sender: String,
    pub receiver: String,
    pub amount: String,
    pub target_chain: String,
    pub fee_token: String,
    pub fee_amount: String,
}