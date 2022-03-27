use std::collections::HashMap;

use cosmwasm_std::{QuerierResult, to_binary};
use mars_core::red_bank::UserAssetDebtResponse;
use mars_core::red_bank::msg::QueryMsg;

#[derive(Default)]
pub struct RedBankQuerier {
    /// Address of mock Red Bank contract to be used in queries
    pub address: String,
    /// Each user's debt info of a specific asset. The 1st key is the user address; the 2nd key is
    /// the asset's label generated by `mars_core::asset::Asset::get_attributes` method.
    pub user_asset_debt: HashMap<(String, String), UserAssetDebtResponse>,
}

impl RedBankQuerier {
    pub fn handle_query(&self, contract_addr: &String, query: QueryMsg) -> QuerierResult {
        if contract_addr != &self.address {
            panic!(
                "[mock]: made a Red Bank query but contract address is incorrect; is {}, should be {}",
                contract_addr,
                self.address
            );
        }

        match query {
            QueryMsg::UserAssetDebt { user_address, asset } => {
                let asset_label = asset.get_attributes().0;
                if let Some(debt) = self.user_asset_debt.get(&(user_address, asset_label)) {
                    Ok(to_binary(debt).into()).into()
                } else {
                    panic!("[mock]: user asset debt is not set");
                }
            },

            _ => panic!("[mock]: Red Bank query is unimplemented")
        }
    }
}