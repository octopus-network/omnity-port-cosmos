use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:omnity-port-cosmos";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        route: msg.route.clone(),
        chain_key: msg.chain_key,
        tokens: BTreeMap::default(),
        handled_tickets: BTreeSet::default(),
        handled_directives: BTreeSet::default(),
        target_chain_factor: BTreeMap::default(),
        fee_token: None,
        fee_token_factor: None,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.route))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = env.contract.address.clone();
    let response = match msg {
        ExecuteMsg::ExecDirective {
            seq,
            directive,
            signature,
        } => execute::exec_directive(deps, env, info, seq, directive, signature),
        ExecuteMsg::PrivilegeMintToken {
            ticket_id,
            token_id,
            receiver,
            amount,
        } => execute::privilege_mint_token(deps, env, info, ticket_id, token_id, receiver, amount),
        ExecuteMsg::RedeemToken {
            token_id,
            receiver,
            amount,
        } => execute::redeem_token(deps, env, info, token_id, receiver, amount),
    }?;
    Ok(response.add_event(Event::new("execute_msg").add_attribute("contract", contract)))
}

pub mod execute {
    use cosmwasm_std::{Addr, Attribute, CosmosMsg, Event, Uint128};
    use prost::Message;

    use crate::{
        cosmos::base::v1beta1::Coin,
        msg::{Directive, Factor},
        osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint},
        state::{read_state, Token},
    };

    use super::*;

    pub fn exec_directive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        seq: u64,
        directive: Directive,
        _sig: Vec<u8>,
    ) -> Result<Response, ContractError> {
        let mut response = Response::new();
        if read_state(deps.storage, |state| {
            state.handled_directives.contains(&seq)
        }) {
            return Err(ContractError::DirectiveAlreadyHandled);
        }
        match directive {
            Directive::AddToken {
                settlement_chain,
                token_id,
                name,
            } => {
                if read_state(deps.storage, |s| s.route != info.sender) {
                    return Err(ContractError::Unauthorized);
                }
                if read_state(deps.storage, |s| s.tokens.contains_key(&token_id)) {
                    return Err(ContractError::TokenAleardyExist);
                }

                let sender = env.contract.address.to_string();
                let denom = format!("factory/{}/{}", sender, name);
                let token = Token {
                    name: name.clone(),
                    denom: denom.clone(),
                    settlement_chain,
                };

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.tokens.insert(token_id.clone(), token);
                    Ok(state)
                })?;

                let msg = MsgCreateDenom {
                    sender,
                    subdenom: name,
                };
                let cosmos_msg = CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".into(),
                    value: Binary::new(msg.encode_to_vec()),
                };

                response = response.add_message(cosmos_msg);
            }
            Directive::UpdateFee { factor } => {
                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    match factor {
                        Factor::FeeTokenFactor {
                            fee_token,
                            fee_token_factor,
                        } => {
                            state.fee_token = Some(fee_token);
                            state.fee_token_factor = Some(fee_token_factor);
                        }
                        Factor::TargetChainFactor {
                            target_chain_id,
                            target_chain_factor,
                        } => {
                            state
                                .target_chain_factor
                                .insert(target_chain_id, target_chain_factor);
                        }
                    }
                    Ok(state)
                })?;
            }
        };
        Ok(response
            .add_event(Event::new("DirectiveExecuted").add_attribute("sequence", seq.to_string())))
    }

    pub fn privilege_mint_token(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        ticket_id: String,
        token_id: String,
        receiver: Addr,
        amount: String,
    ) -> Result<Response, ContractError> {
        if read_state(deps.storage, |s| s.route != info.sender) {
            return Err(ContractError::Unauthorized);
        }

        if read_state(deps.storage, |s| s.handled_tickets.contains(&ticket_id)) {
            return Err(ContractError::TicketAlreadyHandled);
        }

        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.handled_tickets.insert(ticket_id.clone());
            Ok(state)
        })?;

        let msg = MsgMint {
            sender: env.contract.address.to_string(),
            amount: Some(Coin {
                denom: token.denom,
                amount: amount.clone(),
            }),
            mint_to_address: receiver.to_string(),
        };
        let cosmos_msg = CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".into(),
            value: Binary::new(msg.encode_to_vec()),
        };

        Ok(Response::new().add_message(cosmos_msg).add_event(
            Event::new("TokenMinted").add_attributes(vec![
                Attribute::new("ticket_id", ticket_id),
                Attribute::new("token_id", token_id),
                Attribute::new("receiver", receiver),
                Attribute::new("amount", amount),
            ]),
        ))
    }

    pub fn redeem_token(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
        receiver: String,
        amount: String,
    ) -> Result<Response, ContractError> {
        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        let fee_token = read_state(deps.storage, |state| {
            state.fee_token.clone().ok_or(ContractError::FeeHasNotSet)
        })?;

        let fee = calculate_fee(deps, token.settlement_chain)?;
        if info
            .funds
            .iter()
            .find(|coin| coin.denom == fee_token)
            .cloned()
            .map_or(true, |fund| fund.amount < Uint128::from(fee))
        {
            return Err(ContractError::InsufficientFee);
        }

        let msg = MsgBurn {
            sender: env.contract.address.to_string(),
            amount: Some(Coin {
                denom: token.denom,
                amount: amount.clone(),
            }),
            burn_from_address: info.sender.to_string(),
        };
        let cosmos_msg = CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgBurn".into(),
            value: Binary::new(msg.encode_to_vec()),
        };

        Ok(Response::new().add_message(cosmos_msg).add_event(
            Event::new("RedeemRequested").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("sender", info.sender),
                Attribute::new("receiver", receiver),
                Attribute::new("amount", amount),
            ]),
        ))
    }

    fn calculate_fee(deps: DepsMut, target_chain: String) -> Result<u128, ContractError> {
        let fee_factor = read_state(deps.storage, |state| {
            state.fee_token_factor.ok_or(ContractError::FeeHasNotSet)
        })?;
        let chain_factor = read_state(deps.storage, |state| {
            state
                .target_chain_factor
                .get(&target_chain)
                .cloned()
                .ok_or(ContractError::FeeHasNotSet)
        })?;
        Ok(fee_factor * chain_factor)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTokenList => to_json_binary(&query::get_token_list(deps)?),
        QueryMsg::GetFeeInfo => to_json_binary(&query::get_fee_info(deps)?),
    }
}

pub mod query {
    use crate::{
        msg::{GetFeeResponse, GetTokenResponse},
        state::read_state,
    };

    use super::*;

    pub fn get_token_list(deps: Deps) -> StdResult<GetTokenResponse> {
        let tokens = read_state(deps.storage, |state| {
            state
                .tokens
                .iter()
                .map(|(_, token)| token.clone())
                .collect()
        });
        Ok(GetTokenResponse { tokens })
    }

    pub fn get_fee_info(deps: Deps) -> StdResult<GetFeeResponse> {
        Ok(read_state(deps.storage, |state| GetFeeResponse {
            fee_token: state.fee_token.clone(),
            fee_token_factor: state.fee_token_factor.clone(),
            target_chain_factor: state.target_chain_factor.clone(),
        }))
    }
}
