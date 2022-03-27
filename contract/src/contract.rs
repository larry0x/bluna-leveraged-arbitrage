use std::cmp;

use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128,
};
use cw_asset::{Asset, AssetInfo};

use crate::adapters::{Hub, Pair, RedBank};
use crate::config::Config;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StatusResponse};
use crate::state::{CONFIG, MINIMUM_RECEIVE};

//--------------------------------------------------------------------------------------------------
// Instantiate
//--------------------------------------------------------------------------------------------------

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(deps.storage, &msg.check(deps.api)?)?;
    Ok(Response::new())
}

//--------------------------------------------------------------------------------------------------
// Execute
//--------------------------------------------------------------------------------------------------

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ExecuteArb {
            amount,
            minimum_profit,
        } => execute_execute_arb(deps, info, amount, minimum_profit),
        ExecuteMsg::FinializeArb {} => execute_finalize_arb(deps),
    }
}

fn execute_execute_arb(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
    minimum_profit: Decimal,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(StdError::generic_err("sender is not owner"));
    }

    let asset_to_offer = Asset::native("uluna", amount);

    let minimum_receive = amount.checked_add(amount * minimum_profit)?;
    MINIMUM_RECEIVE.save(deps.storage, &minimum_receive)?;

    Ok(Response::new()
        // Borrow Luna of specified amount from Red Bank
        .add_message(RedBank(&config.red_bank).borrow_msg(&asset_to_offer)?)
        // Swap borrowed Luna for bLuna; handle the reply
        .add_submessage(SubMsg::reply_on_success(
            Pair(&config.bluna_pair).swap_msg(&asset_to_offer)?,
            1,
        ))
        .add_attribute("action", "bluna_lev_arb/execute/execute_arb")
        .add_attribute("asset_offered", asset_to_offer.to_string()))
}

fn execute_finalize_arb(deps: DepsMut) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    Ok(Response::new()
        // Withdraw unbonded Luna from bLuna Hub
        .add_submessage(SubMsg::reply_on_success(
            Hub(&config.bluna_hub).withdraw_msg()?,
            2,
        ))
        .add_attribute("action", "bluna_lev_arb/execute/finalize_arb"))
}

//--------------------------------------------------------------------------------------------------
// Reply
//--------------------------------------------------------------------------------------------------

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => after_swap(deps, unwrap_reply(reply)?),
        2 => after_withdrawal(deps, env, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}", id))),
    }
}

fn after_swap(deps: DepsMut, response: SubMsgExecutionResponse) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    let asset_received = Pair(&config.bluna_pair).parse_swap_events(deps.api, &response.events)?;

    let minimum_receive = MINIMUM_RECEIVE.load(deps.storage)?;
    MINIMUM_RECEIVE.remove(deps.storage);

    if asset_received.amount < minimum_receive {
        return Err(StdError::generic_err(
            format!("too little received from swap: {}, expecting at least {}", asset_received, minimum_receive)
        ));
    }

    Ok(Response::new()
        .add_message(Hub(&config.bluna_hub).unbond_msg(&asset_received)?)
        .add_attribute("action", "bluna_lev_arb/reply/after_swap")
        .add_attribute("asset_received", asset_received.to_string()))
}

fn after_withdrawal(
    deps: DepsMut,
    env: Env,
    response: SubMsgExecutionResponse,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // Find how much unbonded Luna was received
    let asset_received = Hub(&config.bluna_hub).parse_withdraw_events(&response.events)?;

    // Query current debt amount and build a message to repay debt
    let debt_info = RedBank(&config.red_bank).query_user_asset_debt(
        &deps.querier,
        &env.contract.address,
        &AssetInfo::native("uluna"),
    )?;
    let asset_to_repay = Asset::native("uluna", cmp::min(asset_received.amount, debt_info.amount));

    // For the remaining assets, we first build messages to send shares to recipients
    // NOTE: Unlike CW20 transfer, `BankMsg` allows zero amount, so we don't need to check whether
    // the amount is zero.
    let amount_available = asset_received.amount - asset_to_repay.amount;
    let mut amount_shared = Uint128::zero();
    let mut msgs: Vec<CosmosMsg> = vec![];
    for (recipient, share) in &config.profit_shares {
        let asset = Asset::native("uluna", amount_available * *share);
        msgs.push(asset.transfer_msg(recipient)?);
        amount_shared += asset.amount;
    }

    // Lastly, send the remaining profit to owner
    let profit = Asset::native("uluna", amount_available - amount_shared);
    msgs.push(profit.transfer_msg(&config.owner)?);

    Ok(Response::new()
        .add_message(RedBank(&config.red_bank).repay_msg(&asset_to_repay)?)
        .add_messages(msgs)
        .add_attribute("action", "bluna_lev_arb/reply/after_withdrawal")
        .add_attribute("asset_received", asset_received.to_string())
        .add_attribute("asset_repaid", asset_to_repay.to_string())
        .add_attribute("profit", profit.to_string()))
}

fn unwrap_reply(reply: Reply) -> StdResult<SubMsgExecutionResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

//--------------------------------------------------------------------------------------------------
// Query
//--------------------------------------------------------------------------------------------------

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Status {} => to_binary(&query_status(deps, env)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config<String>> {
    Ok(CONFIG.load(deps.storage)?.into())
}

fn query_status(deps: Deps, env: Env) -> StdResult<StatusResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(StatusResponse {
        debt: RedBank(&config.red_bank).query_user_asset_debt(
            &deps.querier,
            &env.contract.address,
            &cw_asset::AssetInfoBase::native("uluna"),
        )?,
        unbond_requests: Hub(&config.bluna_hub)
            .query_unbond_requests(&deps.querier, &env.contract.address)?,
        withdrawable_unbonded: Hub(&config.bluna_hub)
            .query_withdrawable_unbonded(&deps.querier, &env.contract.address)?,
    })
}

//--------------------------------------------------------------------------------------------------
// Migrate
//--------------------------------------------------------------------------------------------------

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new())
}
