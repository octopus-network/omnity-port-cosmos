use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::route::ChainState;
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
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        route: msg.route.clone(),
        admin: info.sender,
        tokens: BTreeMap::default(),
        handled_tickets: BTreeSet::default(),
        handled_directives: BTreeSet::default(),
        target_chain_factor: BTreeMap::default(),
        fee_token: None,
        fee_token_factor: None,
        counterparties: BTreeMap::default(),
        chain_id: msg.chain_id,
        chain_state: ChainState::Active
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("route", msg.route))
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
        ExecuteMsg::TestMsg { text } => {
            let msg = format!("{}: {}", info.sender, text);
            Ok(Response::new().add_attribute("message", msg))
        }
        ExecuteMsg::ExecDirective { seq, directive } => {
            execute::exec_directive(deps, env, info, seq, directive)
        }
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
            target_chain
        } => execute::redeem_token(deps, env, info, token_id, receiver, amount, target_chain),
        ExecuteMsg::MintRunes { token_id, receiver, target_chain } => {
            execute::mint_runes(deps, info, token_id, receiver, target_chain)
        }
        ExecuteMsg::BurnToken { token_id, amount, target_chain } => {
            execute::burn_token(deps, env, info, token_id, amount, target_chain)
        }
        ExecuteMsg::UpdateRoute { route } => execute::update_route(deps, info, route),
    }?;
    Ok(response.add_event(Event::new("execute_msg").add_attribute("contract", contract)))
}

pub mod execute {
    use cosmwasm_std::{Addr, Attribute, CosmosMsg, Event, Uint128};
    use prost::Message;

    use crate::{
        cosmos::base::v1beta1::Coin, osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint}, route::{Directive, Factor, Token}, state::read_state
    };

    use super::*;

    pub fn exec_directive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        seq: u64,
        directive: Directive,
    ) -> Result<Response, ContractError> {
        let mut response = Response::new();

        if read_state(deps.storage, |s| s.route != info.sender) {
            return Err(ContractError::Unauthorized);
        }

        if read_state(deps.storage, |state| {
            state.handled_directives.contains(&seq)
        }) {
            return Err(ContractError::DirectiveAlreadyHandled);
        }
        match directive {
            Directive::AddToken(token) => {
                if read_state(deps.storage, |s| s.tokens.contains_key(&token.token_id)) {
                    return Err(ContractError::TokenAleardyExist);
                }

                let sender = env.contract.address.to_string();
                let denom = format!("factory/{}/{}", sender, token.name);

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.tokens.insert(token.token_id.clone(), token.clone());
                    Ok(state)
                })?;

                let msg = MsgCreateDenom {
                    sender,
                    subdenom: token.name,
                };
                let cosmos_msg = CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".into(),
                    value: Binary::new(msg.encode_to_vec()),
                };

                response = response.add_message(cosmos_msg);
            }
            Directive::UpdateFee(factor) => {
                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    match factor {
                        Factor::UpdateFeeTokenFactor(fee_token_factor) => {
                            state.fee_token = Some(fee_token_factor.fee_token);
                            state.fee_token_factor = Some(fee_token_factor.fee_token_factor);
                        }
                        Factor::UpdateTargetChainFactor(target_chain_factor) => {
                            state
                                .target_chain_factor
                                .insert(target_chain_factor.target_chain_id, target_chain_factor.target_chain_factor);
                        }
                    }
                    Ok(state)
                })?;
            }
            Directive::AddChain(chain) | Directive::UpdateChain(chain) => {
                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.counterparties.insert(chain.chain_id.clone(), chain );
                    Ok(state)
                })?; 
            },
            Directive::UpdateToken(_) => todo!(),
            Directive::ToggleChainState(toggle_state) => {

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {

                    if toggle_state.chain_id == state.chain_id {
                        state.chain_state = toggle_state.action.into();
                    } else {
                        let chain = state
                            .counterparties
                            .get_mut(&toggle_state.chain_id)
                            .ok_or(ContractError::ChainNotFound)?;
                        chain.chain_state = toggle_state.action.into();
                    }
                    
                    Ok(state)
                })?;

            },
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
                // todo if token name can always be used as denom
                denom: token.name,
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
        target_chain: String
    ) -> Result<Response, ContractError> {
        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        check_fee(&deps, &info,  target_chain)?;

        let burn_msg = build_burn_msg(
            env.contract.address,
            info.sender.clone(),
            token.name,
            amount.clone(),
        );
        Ok(Response::new().add_message(burn_msg).add_event(
            Event::new("RedeemRequested").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("sender", info.sender),
                Attribute::new("receiver", receiver),
                Attribute::new("amount", amount),
            ]),
        ))
    }

    pub fn mint_runes(
        deps: DepsMut,
        info: MessageInfo,
        token_id: String,
        receiver: Addr,
        target_chain: String
    ) -> Result<Response, ContractError> {
        // let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
        //     Some(token) => Ok(token.clone()),
        //     None => Err(ContractError::TokenNotFound),
        // })?;
        if !token_id.starts_with("Bitcoin-runes-") {
            return Err(ContractError::TokenUnsupportMint);
        }

        check_fee(&deps, &info, target_chain )?;

        Ok(
            Response::new().add_event(Event::new("RunesMintRequested").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("sender", info.sender),
                Attribute::new("receiver", receiver),
            ])),
        )
    }

    pub fn burn_token(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
        amount: String,
        target_chain: String
    ) -> Result<Response, ContractError> {
        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        check_fee(&deps, &info, target_chain)?;

        let burn_msg = build_burn_msg(
            env.contract.address,
            info.sender.clone(),
            token.name,
            amount.clone(),
        );
        Ok(Response::new().add_message(burn_msg).add_event(
            Event::new("TokenBurned").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("sender", info.sender),
                Attribute::new("amount", amount),
            ]),
        ))
    }

    pub fn update_route(
        deps: DepsMut,
        info: MessageInfo,
        route: Addr,
    ) -> Result<Response, ContractError> {
        if read_state(deps.storage, |s| info.sender != s.admin) {
            return Err(ContractError::Unauthorized);
        }

        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.route = route.clone();
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("RouteUpdated").add_attributes(vec![Attribute::new("new_route", route)]),
        ))
    }

    fn build_burn_msg(
        contract_addr: Addr,
        sender: Addr,
        denom: String,
        amount: String,
    ) -> CosmosMsg {
        let msg = MsgBurn {
            sender: contract_addr.to_string(),
            amount: Some(Coin { denom, amount }),
            burn_from_address: sender.to_string(),
        };
        CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgBurn".into(),
            value: Binary::new(msg.encode_to_vec()),
        }
    }

    fn check_fee(
        deps: &DepsMut, 
        info: &MessageInfo, 
        target_chain: String
    ) -> Result<(), ContractError> {
        let fee_token = read_state(deps.storage, |state| {
            state.fee_token.clone().ok_or(ContractError::FeeHasNotSet)
        })?;

        let fee = calculate_fee(deps, target_chain)?;
        if info
            .funds
            .iter()
            .find(|coin| coin.denom == fee_token)
            .cloned()
            .map_or(true, |fund| fund.amount < Uint128::from(fee))
        {
            return Err(ContractError::InsufficientFee);
        }
        Ok(())
    }

    fn calculate_fee(deps: &DepsMut, target_chain: String) -> Result<u128, ContractError> {
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
