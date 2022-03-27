use cosmwasm_std::{Addr, Api, Decimal, StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config<T> {
    /// Owner of this contract
    pub owner: T,
    /// Address of the bLuna token
    pub bluna_token: T,
    /// Address of Astroport bLuna-Luna pair
    pub bluna_pair: T,
    /// Address of Anchor protocol bLuna Hub contract
    pub bluna_hub: T,
    /// Address of Mars protocol lending market contract
    pub red_bank: T,
    /// Accounts to receive portions of the profit, and their percentage shares. The sum of the
    /// shares must be less or equal to one. Remaining profit goes to the owner.
    pub profit_shares: Vec<(T, Decimal)>,
}

impl From<Config<Addr>> for Config<String> {
    fn from(config: Config<Addr>) -> Self {
        Self {
            owner: config.owner.to_string(),
            bluna_token: config.bluna_token.to_string(),
            bluna_pair: config.bluna_pair.to_string(),
            bluna_hub: config.bluna_hub.to_string(),
            red_bank: config.red_bank.to_string(),
            profit_shares: config
                .profit_shares
                .iter()
                .map(|(acct, share)| (acct.to_string(), *share))
                .collect(),
        }
    }
}

impl Config<String> {
    pub fn check(&self, api: &dyn Api) -> StdResult<Config<Addr>> {
        // 1. The sum of the shares must be equal or less than one
        let total_shares: Decimal = self
            .profit_shares
            .iter()
            .fold(Decimal::zero(), |acc, (_, share)| acc + *share);
        if total_shares > Decimal::one() {
            return Err(StdError::generic_err(
                format!("total shares {} is greater than one", total_shares)
            ));
        }

        // 2. All addresses must be valid
        Ok(Config {
            owner: api.addr_validate(&self.owner)?,
            bluna_token: api.addr_validate(&self.bluna_token)?,
            bluna_pair: api.addr_validate(&self.bluna_pair)?,
            bluna_hub: api.addr_validate(&self.bluna_hub)?,
            red_bank: api.addr_validate(&self.red_bank)?,
            profit_shares: self
                .profit_shares
                .iter()
                .map(|(acct, share)| Ok((api.addr_validate(acct)?, *share)))
                .collect::<StdResult<Vec<(Addr, Decimal)>>>()?,
        })
    }
}
