use cosmwasm_std::{Decimal, Empty, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::Config;

pub type InstantiateMsg = Config<String>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Execute an arbitrage operation:
    /// 1. borrow Luna of specified amount from Red Bank
    /// 2. swap Luna for bLuna at Astroport pool
    /// 3. assert that profit (bLuna amount / Luna amount - 1) is greater than minimum profit
    /// 4. unbond bLuna at bLuna Hub
    ExecuteArb {
        amount: Uint128,
        minimum_profit: Decimal,
    },
    /// Once bLuna unbonding is finished,
    /// 1. claim unbonded Luna
    /// 2. repay Luna debt to Red Bank
    /// 3. pay fee to Mars treasury
    /// 4. distribute the remaining reward to owner
    FinializeArb {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The contract's config. Response: `crate::config::Config<String>`
    Config {},
    /// Current status of the contract, including:
    /// - debt owed to Red Bank
    /// - ongoing unbonding requests at bLuna Hub
    /// - withdrawable unbonded amount at bLuna Hub
    /// Respons: `StatusResponse`
    Status {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StatusResponse {
    pub debt: mars_core::red_bank::UserAssetDebtResponse,
    pub unbond_requests: basset::hub::UnbondRequestsResponse,
    pub withdrawable_unbonded: basset::hub::WithdrawableUnbondedResponse,
}

pub type MigrateMsg = Empty;
