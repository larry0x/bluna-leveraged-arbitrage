use std::str::FromStr;

use basset::hub::{
    Cw20HookMsg, ExecuteMsg, QueryMsg, UnbondRequestsResponse, WithdrawableUnbondedResponse,
};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Event, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};
use cw_asset::Asset;

use super::helpers::event_contains_attr;

/// Helper functions for interacting with Anchor protocol's bLuna Hub contract
pub struct Hub<'a>(pub &'a Addr);

impl<'a> Hub<'a> {
    /// Create a message for unbonding specified amount of bLuna
    pub fn unbond_msg(&self, asset: &Asset) -> StdResult<CosmosMsg> {
        asset.send_msg(self.0.to_string(), to_binary(&Cw20HookMsg::Unbond {})?)
    }

    /// Create a `SubMsg` for withdrawing unbonded bLuna
    pub fn withdraw_msg(&self) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawUnbonded {})?,
            funds: vec![],
        }))
    }

    /// When handling response of a withdrawal, parse the events to find out the withdrawn amount
    pub fn parse_withdraw_events(&self, events: &[Event]) -> StdResult<Asset> {
        let event = events
            .iter()
            .find(|event| event_contains_attr(event, "action", "finish_burn"))
            .ok_or_else(|| StdError::generic_err("cannot find `finish_burn` event"))?;

        let withdrawn_amount_str = event
            .attributes
            .iter()
            .cloned()
            .find(|attr| attr.key == "amount")
            .ok_or_else(|| StdError::generic_err("cannot find `amount` attribute"))?
            .value;

        let withdrawn_amount = Uint128::from_str(&withdrawn_amount_str)?;

        Ok(Asset::native("uluna", withdrawn_amount))
    }

    /// Query the user's unbonding requests
    pub fn query_unbond_requests(
        &self,
        querier: &QuerierWrapper,
        user_addr: &Addr,
    ) -> StdResult<UnbondRequestsResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::UnbondRequests {
                address: user_addr.to_string(),
            },
        )
    }

    /// Query the user's withdrawable unbonded Luna
    pub fn query_withdrawable_unbonded(
        &self,
        querier: &QuerierWrapper,
        user_addr: &Addr,
    ) -> StdResult<WithdrawableUnbondedResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::WithdrawableUnbonded {
                address: user_addr.to_string(),
            },
        )
    }
}
