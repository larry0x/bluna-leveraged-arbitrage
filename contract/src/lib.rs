#[cfg(not(feature = "library"))]
pub mod contract;

pub mod adapters;
pub mod config;
pub mod msg;
pub mod state;

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod test_utils;
