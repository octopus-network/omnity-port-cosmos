use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetCountResponse, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use execute::redeem_token;

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
        owner: msg.owner.clone(),
        chain_key: msg.chain_key,
        tokens: BTreeMap::default(),
        handled_tickets: BTreeSet::default(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExecDirective {
            directive,
            signature,
        } => execute::exec_directive(deps, env, info, directive, signature),
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
        } => redeem_token(deps, env, info, token_id, receiver, amount),
    }
}

pub mod execute {
    use cosmwasm_std::{Addr, Attribute, CosmosMsg, Event};
    use prost::Message;

    use crate::{
        cosmos::base::v1beta1::Coin,
        msg::Directive,
        osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint},
        state::{read_state, Token},
    };

    use super::*;

    pub fn exec_directive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        directive: Directive,
        _sig: Vec<u8>,
    ) -> Result<Response, ContractError> {
        match directive {
            Directive::AddToken { token_id, name } => {
                if read_state(deps.storage, |s| s.owner != info.sender) {
                    return Err(ContractError::Unauthorized);
                }
                if read_state(deps.storage, |s| s.tokens.contains_key(&token_id)) {
                    return Err(ContractError::TokenAleardyExist);
                }

                let sender = env.contract.address.to_string();
                let denom = format!("factory/{}/{}", sender, name);

                STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                    state.tokens.insert(token_id.clone(), Token { denom });
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

                Ok(Response::new()
                    .add_message(cosmos_msg)
                    .add_attribute("action", "add token")
                    .add_attribute("token_id", token_id))
            }
        }
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
        if read_state(deps.storage, |s| s.owner != info.sender) {
            return Err(ContractError::Unauthorized);
        }

        let token = read_state(deps.storage, |s| match s.tokens.get(&token_id) {
            Some(token) => Ok(token.clone()),
            None => Err(ContractError::TokenNotFound),
        })?;

        if read_state(deps.storage, |s| s.handled_tickets.contains(&ticket_id)) {
            return Err(ContractError::TicketAlreadyHandled);
        }

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
            Event::new("TokenBurned").add_attributes(vec![
                Attribute::new("token_id", token_id),
                Attribute::new("sender", info.sender),
                Attribute::new("receiver", receiver),
                Attribute::new("amount", amount),
            ]),
        ))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_json_binary(&query::count(deps)?),
    }
}

pub mod query {
    use super::*;

    pub fn count(_deps: Deps) -> StdResult<GetCountResponse> {
        Ok(GetCountResponse { count: 1 })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
//     use cosmwasm_std::{coins, from_json, Addr};

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = message_info(&Addr::unchecked("creator"), &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_json(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

//     #[test]
//     fn increment() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = message_info(&Addr::unchecked("creator"), &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let info = message_info(&Addr::unchecked("anyone"), &coins(2, "token"));
//         let msg = ExecuteMsg::Increment {};
//         let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // should increase counter by 1
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_json(&res).unwrap();
//         assert_eq!(18, value.count);
//     }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = message_info(&Addr::unchecked("creator"), &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauth_info = message_info(&Addr::unchecked("anyone"), &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = message_info(&Addr::unchecked("creator"), &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_json(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
