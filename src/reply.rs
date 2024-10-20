#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Attribute, DepsMut, Env, Event, Reply, Response};

use crate::{
    msg::reply_msg_id,
    state::{GenerateTicketReq, GENERATE_TICKET_REQ},
    ContractError,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        Err(ContractError::ReplyError(msg.result.unwrap_err()))?;
    }
    let msg_id = msg.id;

    match msg_id {
        reply_msg_id::REDEEM_REPLY_ID => {
            let generate_ticket_req: GenerateTicketReq =
                serde_json::from_slice(msg.payload.as_slice())
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
                serde_json::from_slice(msg.payload.as_slice())
                    .map_err(|e| ContractError::CustomError(e.to_string()))?;
            let req_str = serde_json::to_string(&generate_ticket_req).unwrap();
            Ok(Response::new()
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
        _ => {
            unreachable!()
            // ...
        }
    }
}
