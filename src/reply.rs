#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Attribute, BankMsg, Binary, CosmosMsg, DepsMut, Env, Event, Reply, Response, SubMsg, Uint128,
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};

use crate::{
    contract::execute::{build_burn_msg, token_denom},
    msg::{reply_msg_id, ExecuteMsg},
    state::{read_state, GenerateTicketReq, IcpChainKeyToken, GENERATE_TICKET_REQ, STATE},
    types::{MintTokenPayload, RedeemAllBTC},
    ContractError,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_ok() {
        reply_success(deps, env, msg)
    } else {
        reply_error(deps, env, msg)
    }
}

pub fn reply_success(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        reply_msg_id::REDEEM_REPLY_ID => {
            let generate_ticket_req: GenerateTicketReq =
                serde_json::from_slice(msg.payload.clone().as_slice())
                    .map_err(|e| ContractError::CustomError(e.to_string()))?;
            let req_str = serde_json::to_string(&generate_ticket_req).unwrap();
            GENERATE_TICKET_REQ.save(
                deps.storage,
                generate_ticket_req.seq,
                &generate_ticket_req,
            )?;
            Ok(Response::new()
                .add_event(Event::new("RedeemRequested").add_attributes(vec![
                    Attribute::new("token_id", generate_ticket_req.token_id.clone()),
                    Attribute::new("sender", generate_ticket_req.sender.clone()),
                    Attribute::new("receiver", generate_ticket_req.receiver.clone()),
                    Attribute::new("amount", generate_ticket_req.amount.clone()),
                    Attribute::new("target_chain", generate_ticket_req.target_chain_id.clone()),
                ]))
                .add_event(Event::new("GenerateTicketRequested").add_attributes(vec![
                        Attribute::new("generate_ticket_request", req_str),
                        Attribute::new("seq", generate_ticket_req.seq.to_string()),
                        Attribute::new("target_chain_id", generate_ticket_req.target_chain_id),
                        Attribute::new("sender", generate_ticket_req.sender),
                        Attribute::new("receiver", generate_ticket_req.receiver),
                        Attribute::new("token_id", generate_ticket_req.token_id),
                        Attribute::new("amount", generate_ticket_req.amount),
                        Attribute::new(
                            "action",
                            serde_json::to_string(&generate_ticket_req.action)
                                .map_err(|e| ContractError::CustomError(e.to_string()))?,
                        ),
                        Attribute::new("timestamp", generate_ticket_req.timestamp.to_string()),
                        Attribute::new("memo", generate_ticket_req.memo.unwrap_or("".to_string())),
                    ])))
        }
        reply_msg_id::GENERATE_TICKET_REPLY_ID => {
            let generate_ticket_req: GenerateTicketReq =
                serde_json::from_slice(msg.payload.clone().as_slice())
                    .map_err(|e| ContractError::CustomError(e.to_string()))?;
            let req_str = serde_json::to_string(&generate_ticket_req).unwrap();
            Ok(
                Response::new().add_event(Event::new("GenerateTicketRequested").add_attributes(
                    vec![
                    Attribute::new("generate_ticket_request", req_str),
                    Attribute::new("seq", generate_ticket_req.seq.to_string()),
                    Attribute::new("target_chain_id", generate_ticket_req.target_chain_id),
                    Attribute::new("sender", generate_ticket_req.sender),
                    Attribute::new("receiver", generate_ticket_req.receiver),
                    Attribute::new("token_id", generate_ticket_req.token_id),
                    Attribute::new("amount", generate_ticket_req.amount),
                    Attribute::new(
                        "action",
                        serde_json::to_string(&generate_ticket_req.action)
                            .map_err(|e| ContractError::CustomError(e.to_string()))?,
                    ),
                    Attribute::new("timestamp", generate_ticket_req.timestamp.to_string()),
                    Attribute::new("memo", generate_ticket_req.memo.unwrap_or("".to_string())),
                ],
                )),
            )
        }
        reply_msg_id::MINT_TOKEN_REPLY_ID => {
            // swap cbtc to alloy btc
            reply_mint_token(deps, env, msg.clone())
        }
        reply_msg_id::SWAP_CKBTC_TO_ALLBTC_REPLY_ID => {
            // send allbtc to receiver
            reply_swap_ckbtc_to_allbtc(deps, env, msg.clone())
        }
        reply_msg_id::SWAP_ALLBTC_TO_CKBTC_REPLY_ID => {
            reply_swap_allbtc_to_ckbtc(deps, env, msg.clone())
        }
        reply_msg_id::SEND_ALLBTC_REPLY_ID => {
            let mint_token_payload: MintTokenPayload =
                serde_json::from_slice(msg.payload.as_slice())
                    .map_err(|e| ContractError::CustomError(e.to_string()))?;
            return Ok(
                Response::new().add_event(Event::new("TokenMinted").add_attributes(vec![
                    Attribute::new("ticket_id", mint_token_payload.ticket_id),
                    Attribute::new("token_id", mint_token_payload.token_id),
                    Attribute::new("receiver", mint_token_payload.receiver),
                    Attribute::new("amount", mint_token_payload.amount),
                ])),
            );
        }

        _ => {
            unreachable!()
            // ...
        }
    }
}

pub fn reply_error(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        reply_msg_id::REDEEM_REPLY_ID => Ok(Response::new().add_event(Event::new("RedeemFailed"))),
        reply_msg_id::GENERATE_TICKET_REPLY_ID => {
            Ok(Response::new().add_event(Event::new("GenerateTicketFailed")))
        }

        reply_msg_id::SWAP_CKBTC_TO_ALLBTC_REPLY_ID => {
            let mint_ckbtc: MintTokenPayload = serde_json::from_slice(msg.payload.as_slice())
                .map_err(|e| ContractError::CustomError(e.to_string()))?;
            Ok(
                Response::new().add_event(Event::new("SwapCKBTCFailed").add_attributes(vec![
                    Attribute::new("ticket_id", mint_ckbtc.ticket_id),
                    Attribute::new("receiver", mint_ckbtc.receiver),
                    Attribute::new("amount", mint_ckbtc.amount),
                ])),
            )
        }

        reply_msg_id::SWAP_ALLBTC_TO_CKBTC_REPLY_ID => {
            let redeem_allbtc: RedeemAllBTC = serde_json::from_slice(msg.payload.as_slice())
                .map_err(|e| ContractError::CustomError(e.to_string()))?;
            Ok(
                Response::new().add_event(Event::new("SwapAllBTCFailed").add_attributes(vec![
                    Attribute::new("sender", redeem_allbtc.sender),
                    Attribute::new("receiver", redeem_allbtc.receiver),
                    Attribute::new("amount", redeem_allbtc.amount),
                    Attribute::new("target_chain", redeem_allbtc.target_chain),
                    Attribute::new("fee_token", redeem_allbtc.fee_token),
                    Attribute::new("fee_amount", redeem_allbtc.fee_amount),
                ])),
            )
        }

        _ => {
            unreachable!()
        }
    }
}

pub fn reply_mint_token(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mint_token_payload: MintTokenPayload = serde_json::from_slice(msg.payload.as_slice())
        .map_err(|e| ContractError::CustomError(e.to_string()))?;

    let (pool_id, allbtc_denom, ckbtc_token_id) = read_state(deps.storage, |s| {
        (
            s.allbtc_swap_pool_id.clone(),
            s.allbtc_token_denom.clone(),
            s.ckbtc_token_id.clone(),
        )
    });

    let mint_token_denom = token_denom(
        env.contract.address.to_string(),
        mint_token_payload.token_id.clone(),
    );

    if mint_token_payload.transmuter.is_none() {
        return Ok(
            Response::new().add_event(Event::new("TokenMinted").add_attributes(vec![
                Attribute::new("ticket_id", mint_token_payload.ticket_id),
                Attribute::new("token_id", mint_token_payload.token_id),
                Attribute::new("receiver", mint_token_payload.receiver),
                Attribute::new("amount", mint_token_payload.amount),
            ])),
        );
    } else {
        if mint_token_payload.token_id.ne(&ckbtc_token_id)
            || mint_token_payload
                .transmuter
                .clone()
                .unwrap()
                .ne(&allbtc_denom)
        {
            return Err(ContractError::CustomError(
                "Only Support transmuter ckbtc to allBTC".to_string(),
            ));
        }
    }

    // swap ckbtc to alloy btc
    let msg = MsgSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        routes: vec![SwapAmountInRoute {
            pool_id: pool_id,
            token_out_denom: allbtc_denom,
        }],
        token_in: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: mint_token_denom,
            amount: mint_token_payload.amount.clone(),
        }),
        token_out_min_amount: mint_token_payload.amount.clone(),
    };

    let cosmos_msg = CosmosMsg::Stargate {
        type_url: "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountIn".into(),
        value: Binary::new(msg.to_proto_bytes()),
    };

    Ok(Response::new().add_submessage(
        SubMsg::reply_on_error(cosmos_msg, reply_msg_id::SWAP_CKBTC_TO_ALLBTC_REPLY_ID)
            .with_payload(
                serde_json::to_vec(&mint_token_payload)
                    .map_err(|e| ContractError::CustomError(e.to_string()))?,
            ),
    ))
}

fn reply_swap_allbtc_to_ckbtc(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let redeem_allbtc: RedeemAllBTC = serde_json::from_slice(msg.payload.as_slice())
        .map_err(|e| ContractError::CustomError(e.to_string()))?;

    let (_pool_id, _allbtc_denom, ckbtc_token_id) = read_state(deps.storage, |s| {
        (
            s.allbtc_swap_pool_id.clone(),
            s.allbtc_token_denom.clone(),
            s.ckbtc_token_id.clone(),
        )
    });

    let ckbtc_denom = token_denom(env.contract.address.to_string(), ckbtc_token_id.clone());

    // redeem ckbtc
    let burn_msg = build_burn_msg(
        env.contract.address.clone(),
        env.contract.address.clone(),
        ckbtc_denom,
        redeem_allbtc.amount.clone(),
    );

    let mut state = STATE.load(deps.storage).expect("State not initialized!");
    let current_seq = state.generate_ticket_sequence;
    state.generate_ticket_sequence += 1;
    STATE.save(deps.storage, &state)?;

    let req = GenerateTicketReq {
        seq: current_seq,
        target_chain_id: redeem_allbtc.target_chain,
        sender: redeem_allbtc.sender,
        receiver: redeem_allbtc.receiver,
        token_id: ckbtc_token_id,
        amount: redeem_allbtc.amount.clone(),
        action: crate::state::TxAction::RedeemIcpChainKeyAssets(IcpChainKeyToken::CKBTC),
        timestamp: env.block.time.nanos(),
        block_height: env.block.height,
        memo: None,
        fee_token: redeem_allbtc.fee_token,
        fee_amount: redeem_allbtc.fee_amount,
    };

    Ok(Response::new().add_submessage(
        SubMsg::reply_on_success(burn_msg, reply_msg_id::REDEEM_REPLY_ID).with_payload(
            serde_json::to_vec(&req).map_err(|e| ContractError::CustomError(e.to_string()))?,
        ),
    ))
}

fn reply_swap_ckbtc_to_allbtc(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    // send allbtc to receiver

    let mint_token_payload: MintTokenPayload = serde_json::from_slice(msg.payload.as_slice())
        .map_err(|e| ContractError::CustomError(e.to_string()))?;

    let all_btc_denom = read_state(deps.storage, |s| s.allbtc_token_denom.clone());

    let bank_msg = BankMsg::Send {
        to_address: mint_token_payload.receiver.clone().into_string(),
        amount: vec![cosmwasm_std::Coin {
            denom: token_denom(env.contract.address.to_string(), all_btc_denom),
            amount: Uint128::new(mint_token_payload.amount.clone().parse().unwrap()),
        }],
    };

    let cosmos_msg = CosmosMsg::Bank(bank_msg);

    Ok(Response::new().add_submessage(
        SubMsg::reply_on_success(cosmos_msg, reply_msg_id::SEND_ALLBTC_REPLY_ID).with_payload(
            serde_json::to_vec(&mint_token_payload)
                .map_err(|e| ContractError::CustomError(e.to_string()))?,
        ),
    ))
}
