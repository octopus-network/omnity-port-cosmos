use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetTargetChainFeeResponse, InstantiateMsg, QueryMsg};
use crate::route::ChainState;
use crate::state::{read_state, State, STATE};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Response, StdError, StdResult
};
use cw2::set_contract_version;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:omnity-port-cosmos";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let new_semver_version: Version = CONTRACT_VERSION.parse()?;
    let old_contract_version = cw2::get_contract_version(deps.storage)?;
    let old_semver_version: Version = old_contract_version.version.parse()?;
    // ensure we are migrating from an allowed contract
    if old_contract_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }
    // note: better to do proper semver compare, but string compare *usually* works
    if old_semver_version >= new_semver_version {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }
    
    // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    // do any desired state migrations...
    
    Ok(Response::default())
}

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
        chain_state: ChainState::Active,
        target_chain_redeem_min_amount: BTreeMap::default(),
        generate_ticket_sequence: 0,
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
    let chain_state_active = read_state(deps.storage, |state| {
        state.chain_state == ChainState::Active
    });
    if !chain_state_active {
        return Err(ContractError::ChainDeactive);
    }
    let contract = env.contract.address.clone();
    let response = match msg {
        ExecuteMsg::ExecDirective { seq, directive } => {
            execute::exec_directive(deps, env, info, seq, directive)
        }
        ExecuteMsg::PrivilegeMintToken {
            ticket_id,
            token_id,
            receiver,
            amount,
        } => execute::privilege_mint_token(
            deps, env, info, ticket_id, token_id, receiver, amount
        ),
        ExecuteMsg::RedeemToken {
            token_id,
            receiver,
            amount,
            target_chain,
        } => execute::redeem_token(
            deps, env, info, token_id, receiver, amount, target_chain
        ),
        ExecuteMsg::GenerateTicket { 
            token_id, 
            sender, 
            receiver, 
            amount, 
            target_chain, 
            action, 
            memo 
        } => {
            execute::generate_ticket(
                deps, env, info, token_id, sender, receiver, amount, target_chain, action, memo
            )
        }
        ExecuteMsg::UpdateRoute { route } => execute::update_route(deps, info, route),
        ExecuteMsg::RedeemSetting { token_id, target_chain, min_amount } => 
        execute::redeem_setting(deps,  info, token_id, target_chain, min_amount),
    }?;
    Ok(response.add_event(Event::new("execute_msg").add_attribute("contract", contract)))
}

pub mod execute {
    use cosmwasm_std::{Addr, Attribute, CosmosMsg, Event, SubMsg};
    use prost::Message;

    use crate::{
        cosmos::{
            bank::v1beta1::{DenomUnit, Metadata},
            base::v1beta1::Coin,
        }, msg::reply_msg_id, osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint, MsgSetDenomMetadata}, route::{Directive, Factor}, state::{read_state, GenerateTicketReq, IcpChainKeyToken}
    };

    use super::*;

    pub fn token_denom(address: String, token_id: String) -> String {
        let denom = format!("factory/{}/{}", address, token_id);
        return denom;
    }

    pub fn exec_directive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        seq: u64,
        directive: Directive,
    ) -> Result<Response, ContractError> {
        let mut response = Response::new();

        if read_state(deps.storage, |s| s.route != info.sender && s.admin != info.sender) {
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
                // let denom = format!("factory/{}/{}", sender, token.name);

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.tokens.insert(token.token_id.clone(), token.clone());
                    Ok(state)
                })?;

                let msg = MsgCreateDenom {
                    sender: sender.clone(),
                    subdenom: token.token_id.clone(),
                };
                let cosmos_msg = CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".into(),
                    value: Binary::new(msg.encode_to_vec()),
                };

                let token_base_denom =
                    token_denom(env.contract.address.to_string(), token.token_id);
                let set_denom_metadata_msg = MsgSetDenomMetadata {
                    sender: sender.clone(),
                    metadata: Some(Metadata {
                        description: token.name.clone(),
                        denom_units: vec![
                            DenomUnit {
                                denom: token_base_denom.clone(),
                                exponent: 0,
                                aliases: vec![],
                            },
                            DenomUnit {
                                denom: token.symbol.clone(),
                                exponent: token.decimals as u32,
                                aliases: vec![],
                            },
                        ],
                        base: token_base_denom,
                        display: token.symbol.clone(),
                        name: token.name,
                        symbol: token.symbol,
                        uri: token.icon.unwrap_or("".to_string()),
                        uri_hash: "".to_string(),
                    }),
                };
                let update_msg = CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgSetDenomMetadata".to_string(),
                    value: set_denom_metadata_msg.encode_to_vec().into(),
                };

                response = response.add_message(cosmos_msg).add_message(update_msg);
            }
            Directive::UpdateFee(factor) => {
                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    match factor {
                        Factor::UpdateFeeTokenFactor(fee_token_factor) => {
                            state.fee_token = Some(fee_token_factor.fee_token);
                            state.fee_token_factor = Some(fee_token_factor.fee_token_factor);
                        }
                        Factor::UpdateTargetChainFactor(target_chain_factor) => {
                            state.target_chain_factor.insert(
                                target_chain_factor.target_chain_id,
                                target_chain_factor.target_chain_factor,
                            );
                        }
                    }
                    Ok(state)
                })?;
            }
            Directive::AddChain(chain) | Directive::UpdateChain(chain) => {
                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    if chain.chain_id == state.chain_id {
                        state.chain_state = chain.chain_state.clone();
                    }

                    state.counterparties.insert(chain.chain_id.clone(), chain);
                    Ok(state)
                })?;
            }
            Directive::UpdateToken(token) => {
                if read_state(deps.storage, |s| !s.tokens.contains_key(&token.token_id)) {
                    return Err(ContractError::TokenNotFound);
                }

                let sender = env.contract.address.to_string();

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.tokens.insert(token.token_id.clone(), token.clone());
                    Ok(state)
                })?;

                let token_base_denom =
                    token_denom(env.contract.address.to_string(), token.token_id);
                let set_denom_metadata_msg = MsgSetDenomMetadata {
                    sender: sender.clone(),
                    metadata: Some(Metadata {
                        description: token.name.clone(),
                        denom_units: vec![
                            DenomUnit {
                                denom: token_base_denom.clone(),
                                exponent: 0,
                                aliases: vec![],
                            },
                            DenomUnit {
                                denom: token.symbol.clone(),
                                exponent: token.decimals as u32,
                                aliases: vec![],
                            },
                        ],
                        base: token_base_denom,
                        display: token.symbol.clone(),
                        name: token.name,
                        symbol: token.symbol,
                        uri: token.icon.unwrap_or("".to_string()),
                        uri_hash: "".to_string(),
                    }),
                };
                let update_msg = CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgSetDenomMetadata".to_string(),
                    value: set_denom_metadata_msg.encode_to_vec().into(),
                };

                response = response.add_message(update_msg);
            }
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
            }
        };

        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.handled_directives.insert(seq);
            Ok(state)
        })?;
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
        if read_state(deps.storage, |s| s.route != info.sender && s.admin != info.sender) {
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

        let denom = token_denom(env.contract.address.to_string(), token.token_id);

        let msg = MsgMint {
            sender: env.contract.address.to_string(),
            amount: Some(Coin {
                denom: denom.clone(),
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
        target_chain: String,
    ) -> Result<Response, ContractError> {
        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        check_target_chain(&deps, target_chain.clone())?;
        check_fee(&deps, &info, target_chain.clone())?;
        check_min_amount(&deps, &token_id, &target_chain, &amount)?;

        let denom = token_denom(env.contract.address.to_string(), token.token_id);

        let burn_msg = build_burn_msg(
            env.contract.address,
            info.sender.clone(),
            denom,
            amount.clone(),
        );

        let mut state = STATE.load(deps.storage).expect("State not initialized!");
        let current_seq = state.generate_ticket_sequence;
        state.generate_ticket_sequence += 1;
        STATE.save(deps.storage, &state)?;
        
        let req = GenerateTicketReq {
            seq: current_seq,
            target_chain_id: target_chain,
            sender: info.sender.into_string(),
            receiver,
            token_id,
            amount: amount.clone(),
            action: crate::state::TxAction::RedeemIcpChainKeyAssets(IcpChainKeyToken::CKBTC),
            timestamp: env.block.time.nanos(),
            block_height: env.block.height,
            memo: None,
        };

        Ok(Response::new()
        .add_submessage(
            SubMsg::reply_on_success(
                burn_msg, 
                reply_msg_id::REDEEM_REPLY_ID
            )
            .with_payload(serde_json::to_vec(&req)
            .map_err(|e| ContractError::CustomError(e.to_string()))?)
        ))
    }

    pub fn generate_ticket(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
        sender: String,
        receiver: String,
        amount: String,
        target_chain: String,
        action: crate::state::TxAction,
        memo: Option<String>,
    )-> Result<Response, ContractError>{
        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        check_target_chain(&deps, target_chain.clone())?;
        check_fee(&deps, &info, target_chain.clone())?;
        check_min_amount(&deps, &token_id, &target_chain, &amount)?;

        let denom = token_denom(env.contract.address.to_string(), token.token_id);

        let burn_msg = build_burn_msg(
            env.contract.address,
            info.sender.clone(),
            denom,
            amount.clone(),
        );

        let mut state = STATE.load(deps.storage).expect("State not initialized!");
        let current_seq = state.generate_ticket_sequence;
        state.generate_ticket_sequence += 1;
        STATE.save(deps.storage, &state)?;

        let generate_ticket_req = GenerateTicketReq {
            seq: current_seq,
            target_chain_id: target_chain,
            sender,
            receiver,
            token_id,
            amount,
            action,
            timestamp: env.block.time.nanos(),
            block_height: env.block.height,
            memo,
        };

        Ok(Response::new()
        .add_submessage(
            SubMsg::reply_on_success(
                burn_msg, 
                reply_msg_id::GENERATE_TICKET_REPLY_ID
            )
            .with_payload(serde_json::to_vec(&generate_ticket_req)
            .map_err(|e| ContractError::CustomError(e.to_string()))?)
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

    pub fn redeem_setting(
        deps: DepsMut,
        info: MessageInfo,
        token_id: String, 
        target_chain: String, 
        min_amount: String
    ) -> Result<Response, ContractError>{
        if read_state(deps.storage, |s| info.sender != s.admin) {
            return Err(ContractError::Unauthorized);
        }

        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.target_chain_redeem_min_amount
                .insert((token_id.clone(), target_chain.clone()), min_amount.clone());
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("RedeemSettingUpdated").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("target_chain", target_chain),
                Attribute::new("min_amount", min_amount),
            ]),
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

    fn check_min_amount(
        deps: &DepsMut,
        token_id: &String,
        target_chain: &String,
        amount: &String,
    ) -> Result<(), ContractError> {
        let min_amount = read_state(deps.storage, |state| {
            state
                .target_chain_redeem_min_amount
                .get(&(token_id.clone(), target_chain.clone()))
                .cloned()
                .unwrap_or("0".to_string())
        });

        if amount.parse::<u128>().unwrap() < min_amount.parse::<u128>().unwrap() {
            return Err(ContractError::RedeemAmountLessThanMinAmount(min_amount, amount.clone()));
        }

        Ok(())
    }

    fn check_target_chain(
        deps: &DepsMut,
        target_chain: String,
    ) -> Result<(), ContractError> {
        let counterparties = read_state(deps.storage, |state| state.counterparties.clone());

        if let Some(target_chain) = counterparties.get(&target_chain) {
            if target_chain.chain_state == ChainState::Active {
                return Ok(());
            } else {
                return Err(ContractError::TargetChainDeactive);
            }
        } else {
            return Err(ContractError::TargetChainNotFound);
        }
    }

    fn check_fee(
        deps: &DepsMut,
        info: &MessageInfo,
        target_chain: String,
    ) -> Result<(), ContractError> {
        let fee_token = read_state(deps.storage, |state| {
            state.fee_token.clone().ok_or(ContractError::FeeHasNotSet)
        })?;

        let fee = calculate_fee(deps, target_chain)?;
        let funds_info = format!("{:?}",info.funds);
        let attached_fee = info
            .funds
            .iter()
            .find(|coin| coin.denom == fee_token)
            .map(|c| c.amount.u128()).unwrap_or(0);
        if attached_fee < fee {
            return Err(ContractError::InsufficientFee(fee, attached_fee, funds_info));
        }
        
        Ok(())
    }

    pub fn calculate_fee(deps: &DepsMut, target_chain: String) -> Result<u128, ContractError> {
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
        QueryMsg::GetTokenList {} => to_json_binary(&query::get_token_list(deps)?),
        QueryMsg::GetFeeInfo {} => to_json_binary(&query::get_fee_info(deps)?),
        QueryMsg::GetTargetChainFee { target_chain } => {
            let fee_info = query::get_fee_info(deps)?;
            if fee_info.fee_token.is_none() {
                return to_json_binary(&GetTargetChainFeeResponse {
                    target_chain: target_chain,
                    fee_token: None,
                    fee_token_factor: None,
                    fee_amount: None,
                });
            }
            let fee_token = fee_info.fee_token.unwrap();
            let fee_token_factor = fee_info.fee_token_factor.unwrap();

            let fee_factor = read_state(deps.storage, |state| {
                state.fee_token_factor.ok_or(ContractError::FeeHasNotSet)
            }).unwrap();
            let chain_factor = read_state(deps.storage, |state| {
                state
                    .target_chain_factor
                    .get(&target_chain)
                    .cloned()
                    .ok_or(ContractError::FeeHasNotSet)
            }).unwrap();
            let fee_amount = fee_factor * chain_factor;

            to_json_binary(&GetTargetChainFeeResponse {
                target_chain: target_chain,
                fee_token: Some(fee_token),
                fee_token_factor: Some(fee_token_factor),
                fee_amount: Some(fee_amount),
            })
        }
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
