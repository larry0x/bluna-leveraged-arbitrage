use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

use crate::config::Config;

/// The configurations of this contract
pub const CONFIG: Item<Config<Addr>> = Item::new("config");

/// The minimum amount of bLuna to receive after a swap. We need to temporarily save it in storage
/// so that it can be accessed when handling the submsg execution result.
pub const MINIMUM_RECEIVE: Item<Uint128> = Item::new("minimum_receive");
