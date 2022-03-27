use std::convert::TryInto;

use cosmwasm_std::{to_binary, Addr, CosmosMsg, QuerierWrapper, StdResult, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo};
use mars_core::red_bank::msg::{ExecuteMsg, QueryMsg, ReceiveMsg};
use mars_core::red_bank::UserAssetDebtResponse;

/// Helper functions for interacting with Mars protocol's Red Bank contract
pub struct RedBank<'a>(pub &'a Addr);

impl<'a> RedBank<'a> {
    /// Create a `SubMsg` to borrow the specified asset from Red Bank
    pub fn borrow_msg(&self, asset: &Asset) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::Borrow {
                asset: asset.info.clone().into(),
                amount: asset.amount,
                recipient: None,
            })?,
            funds: vec![],
        }))
    }

    /// Create a message to repay the specified asset to Red Bank
    pub fn repay_msg(&self, asset: &Asset) -> StdResult<CosmosMsg> {
        Ok(match &asset.info {
            AssetInfo::Cw20(_) => asset.send_msg(
                self.0,
                to_binary(&Cw20ExecuteMsg::Send {
                    contract: self.0.to_string(),
                    amount: asset.amount,
                    msg: to_binary(&ReceiveMsg::RepayCw20 { on_behalf_of: None })?,
                })?,
            )?,
            AssetInfo::Native(denom) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_binary(&ExecuteMsg::RepayNative {
                    denom: denom.into(),
                    on_behalf_of: None,
                })?,
                funds: vec![asset.try_into()?],
            }),
        })
    }

    /// Query the user's debt of the specified asset at Red Bank
    pub fn query_user_asset_debt(
        &self,
        querier: &QuerierWrapper,
        user_addr: &Addr,
        asset_info: &AssetInfo,
    ) -> StdResult<UserAssetDebtResponse> {
        querier.query_wasm_smart(
            self.0.clone(),
            &QueryMsg::UserAssetDebt {
                user_address: user_addr.to_string(),
                asset: asset_info.into(),
            },
        )
    }
}
