use std::collections::HashMap;

use msg::ExecuteMsg;

use crate::*;

pub type ChainId = String;
pub type TokenId = String;

#[cw_serde]
pub struct Token {
    pub token_id: String,
    pub name: String,
    pub symbol: String,

    pub decimals: u8,
    pub icon: Option<String>,
    pub metadata: HashMap<String, String>,
}
impl Eq for Token {}

#[cw_serde]
pub enum Directive {
    AddChain(Chain),
    AddToken(Token),
    UpdateChain(Chain),
    UpdateToken(Token),
    ToggleChainState(ToggleState),
    UpdateFee(Factor),
}

#[cw_serde]
pub struct Chain {
    pub chain_id: ChainId,
    pub canister_id: String,
    pub chain_type: ChainType,
    // the chain default state is true
    pub chain_state: ChainState,
    // settlement chain: export contract address
    // execution chain: port contract address
    pub contract_address: Option<String>,

    // optional counterparty chains
    pub counterparties: Option<Vec<ChainId>>,
    // fee token
    pub fee_token: Option<TokenId>,
}

// impl PartialEq for Chain {
//     fn eq(&self, other: &Self) -> bool {
//         self.chain_id == other.chain_id
//     }
// }

impl Eq for Chain {}

#[cw_serde]
pub struct ToggleState {
    pub chain_id: ChainId,
    pub action: ToggleAction,
}

#[cw_serde]
pub enum ToggleAction {
    Activate,
    Deactivate,
}

impl From<ToggleAction> for ChainState {
    fn from(value: ToggleAction) -> Self {
        match value {
            ToggleAction::Activate => ChainState::Active,
            ToggleAction::Deactivate => ChainState::Deactive,
        }
    }
}

#[cw_serde]
pub enum ChainType {
    SettlementChain,
    ExecutionChain,
}

#[cw_serde]
pub enum ChainState {
    Active,
    Deactive,
}

impl Eq for ChainState {}

#[cw_serde]
pub enum Factor {
    UpdateTargetChainFactor(TargetChainFactor),
    UpdateFeeTokenFactor(FeeTokenFactor),
}

#[cw_serde]
pub struct TargetChainFactor {
    pub target_chain_id: ChainId,
    pub target_chain_factor: u128,
}

#[cw_serde]
pub struct FeeTokenFactor {
    pub fee_token: TokenId,
    pub fee_token_factor: u128,
}

#[test]
pub fn test_token_cw_serde() {
    let token = Token {
        token_id: "1".to_string(),
        name: "Rune".to_string(),
        symbol: "RUNE".to_string(),
        decimals: 18,
        icon: None,
        metadata: HashMap::new(),
    };

    let msg = ExecuteMsg::ExecDirective { 
        seq: 0, 
        directive: Directive::AddToken(token.clone())  
    };

    let s = serde_json::to_string(&msg).unwrap();
    dbg!(&s);
    

}