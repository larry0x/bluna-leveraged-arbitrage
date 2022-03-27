use cosmwasm_std::testing::MockQuerier;
use cosmwasm_std::{
    from_binary, from_slice, Empty, Querier, QuerierResult, QueryRequest, SystemError, WasmQuery,
};

use mars_core::asset::Asset;
use mars_core::red_bank::msg::QueryMsg as RedBankQueryMsg;
use mars_core::red_bank::UserAssetDebtResponse;

use super::RedBankQuerier;

pub struct CustomQuerier {
    base: MockQuerier<Empty>,
    red_bank_querier: RedBankQuerier,
}

impl Querier for CustomQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
                .into()
            },
        };
        self.handle_query(&request)
    }
}

impl Default for CustomQuerier {
    fn default() -> Self {
        Self {
            base: MockQuerier::new(&[]),
            red_bank_querier: RedBankQuerier::default(),
        }
    }
}

impl CustomQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                if let Ok(red_bank_query) = from_binary::<RedBankQueryMsg>(msg) {
                    return self
                        .red_bank_querier
                        .handle_query(contract_addr, red_bank_query);
                }

                panic!("[mock]: unsupported wasm query: {:?}", msg);
            },

            _ => self.base.handle_query(request),
        }
    }

    pub fn set_red_bank_address<T: Into<String>>(&mut self, address: T) {
        self.red_bank_querier.address = address.into();
    }

    pub fn set_red_bank_user_debt<T: Into<String>>(
        &mut self,
        user_address: T,
        asset: Asset,
        debt: UserAssetDebtResponse,
    ) {
        self.red_bank_querier
            .user_asset_debt
            .insert((user_address.into(), asset.get_attributes().0), debt);
    }
}
