use std::convert::TryInto;
use std::str::FromStr;

use astroport::pair::{Cw20HookMsg, ExecuteMsg};
use cosmwasm_std::{to_binary, Addr, Api, CosmosMsg, Event, StdError, StdResult, Uint128, WasmMsg};
use cw_asset::{Asset, AssetInfo};

use super::helpers::event_contains_attr;

/// Helper function for interacting with Astroport pair contract
pub struct Pair<'a>(pub &'a Addr);

impl<'a> Pair<'a> {
    /// Create a `SubMsg` that swaps the specified asset
    pub fn swap_msg(&self, asset: &Asset) -> StdResult<CosmosMsg> {
        match &asset.info {
            AssetInfo::Cw20(_) => asset.send_msg(
                self.0,
                to_binary(&Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })?,
            ),
            AssetInfo::Native(_) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_binary(&ExecuteMsg::Swap {
                    offer_asset: asset.clone().into(),
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })?,
                funds: vec![asset.try_into()?],
            })),
        }
    }

    /// When handling the response of a swap, parse the events to find out the returned asset and its amount
    pub fn parse_swap_events(&self, api: &dyn Api, events: &[Event]) -> StdResult<Asset> {
        let event = events
            .iter()
            .find(|event| event_contains_attr(event, "action", "swap"))
            .ok_or_else(|| StdError::generic_err("cannot find `swap` event"))?;

        let ask_asset_str = event
            .attributes
            .iter()
            .cloned()
            .find(|attr| attr.key == "ask_asset")
            .ok_or_else(|| StdError::generic_err("cannot find `ask_asset` attribute"))?
            .value;

        let return_amount_str = event
            .attributes
            .iter()
            .cloned()
            .find(|attr| attr.key == "return_amount")
            .ok_or_else(|| StdError::generic_err("cannot find `return_amount` attribute"))?
            .value;

        let return_amount = Uint128::from_str(&return_amount_str)?;

        // If the asset's label can be parsed into an `Addr`, then we assume it is a CW20; otherwise,
        // we assume it is a native coin.
        //
        // Not a perfectly safe implementation; as native coins can have arbitrary denoms, it is
        // possible to create an native coin whose denom is a valid Terra address. However, since
        // Terra does not allow minting arbitrary native coins, this risk is clsoe to non-existent.
        //
        // If only Astroport had used `cw-asset`... There wouldn't have been this ambiguity!
        let return_asset = match api.addr_validate(&ask_asset_str) {
            Ok(contract_addr) => Asset::cw20(contract_addr, return_amount),
            _ => Asset::native(ask_asset_str, return_amount),
        };

        Ok(return_asset)
    }
}
