use std::str::FromStr;

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Coin, ContractResult, CosmosMsg, Decimal, Deps, Event,
    OwnedDeps, Reply, ReplyOn, StdError, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use mars_core::asset::{Asset as LegacyAsset, AssetType as LegacyAssetType};
use mars_core::red_bank::UserAssetDebtResponse;
use serde::de::DeserializeOwned;

use crate::config::Config;
use crate::contract::{execute, instantiate, query, reply};
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::MINIMUM_RECEIVE;
use crate::test_utils::CustomQuerier;

fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: CustomQuerier::default(),
    }
}

fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}

fn create_config() -> Config<Addr> {
    Config {
        owner: Addr::unchecked("owner"),
        bluna_token: Addr::unchecked("bluna_token"),
        bluna_pair: Addr::unchecked("bluna_pair"),
        bluna_hub: Addr::unchecked("bluna_hub"),
        red_bank: Addr::unchecked("red_bank"),
        profit_shares: vec![
            (Addr::unchecked("alice"), Decimal::from_str("0.2").unwrap()),
            (Addr::unchecked("bob"), Decimal::from_str("0.1").unwrap()),
        ],
    }
}

fn setup_test() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    let mut deps = mock_dependencies();

    // Instantiate contract
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        create_config().into(),
    )
    .unwrap();

    // Set up user asset debt at Red Bank
    let asset = LegacyAsset::Native {
        denom: String::from("uluna"),
    };
    let debt = UserAssetDebtResponse {
        denom: String::from("uluna"),
        asset_label: String::from("uluna"),
        asset_reference: String::from("uluna").as_bytes().to_vec(),
        asset_type: LegacyAssetType::Native,
        amount_scaled: Uint128::new(100_000_000_000),
        amount: Uint128::new(101_000_000_000), // assume 1 Luna new debt
    };
    deps.querier.set_red_bank_address("red_bank");
    deps.querier.set_red_bank_user_debt(MOCK_CONTRACT_ADDR, asset, debt);

    deps
}

#[test]
fn proper_instantiation() {
    let mut deps = setup_test();

    // Invalid config: The sum of shares cannot be greater than one
    let mut invalid_config = create_config();
    invalid_config.profit_shares.push((
        Addr::unchecked("charlie"),
        Decimal::from_str("0.8").unwrap(),
    ));

    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        invalid_config.into(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("total shares 1.1 is greater than one")
    );

    // Valid config: The config should have been saved in storage and can be queried
    let res: Config<String> = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(res, create_config().into());
}

#[test]
fn executing_arb() {
    let mut deps = setup_test();

    let msg = ExecuteMsg::ExecuteArb {
        amount: Uint128::new(100_000_000_000),
        minimum_profit: Decimal::from_str("0.05").unwrap(),
    };

    // Non-owner cannot call
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("non_owner", &[]),
        msg.clone(),
    )
    .unwrap_err();
    assert_eq!(err, StdError::generic_err("sender is not owner"));

    // Owner can call
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("red_bank"),
                msg: to_binary(&mars_core::red_bank::msg::ExecuteMsg::Borrow {
                    asset: mars_core::asset::Asset::Native {
                        denom: String::from("uluna")
                    },
                    amount: Uint128::new(100_000_000_000),
                    recipient: None,
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("bluna_pair"),
                msg: to_binary(&astroport::pair::ExecuteMsg::Swap {
                    offer_asset: astroport::asset::Asset {
                        info: astroport::asset::AssetInfo::NativeToken {
                            denom: String::from("uluna")
                        },
                        amount: Uint128::new(100_000_000_000)
                    },
                    belief_price: None,
                    max_spread: None,
                    to: None
                })
                .unwrap(),
                funds: vec![Coin::new(100_000_000_000, "uluna")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success
        }
    );

    // Minimum receive amount should have been saved
    let minimum_receive = MINIMUM_RECEIVE.load(deps.as_ref().storage).unwrap();
    assert_eq!(minimum_receive, Uint128::new(105_000_000_000));
}

#[test]
fn replying_after_swap() {
    let mut deps = setup_test();

    //------------------------------------------------------------
    // Test 1. Return amount is smaller than minimum receive
    //
    MINIMUM_RECEIVE
        .save(deps.as_mut().storage, &Uint128::new(105_000_000_000))
        .unwrap();

    let invalid_response = SubMsgExecutionResponse {
        events: vec![
            Event::new("from_contract")
                .add_attribute("action", "swap")
                .add_attribute("offer_asset", "uluna")
                .add_attribute("ask_asset", "bluna_token")
                .add_attribute("offer_amount", "100000000000")
                .add_attribute("return_amount", "102500000000"), // less than 5% profit
        ],
        data: None,
    };

    let err = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 1,
            result: ContractResult::Ok(invalid_response.clone()),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("too little received from swap: cw20:bluna_token:102500000000, expecting at least 105000000000")
    );

    //------------------------------------------------------------
    // Test 2. Return amount is greater than minimum receive
    //
    MINIMUM_RECEIVE
        .save(deps.as_mut().storage, &Uint128::new(105_000_000_000))
        .unwrap();

    let mut valid_response = invalid_response.clone();
    valid_response.events[0].attributes[4].value = String::from("108000000000");

    let res = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 1,
            result: ContractResult::Ok(valid_response),
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("bluna_token"),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: String::from("bluna_hub"),
                    amount: Uint128::new(108000000000),
                    msg: to_binary(&basset::hub::Cw20HookMsg::Unbond {}).unwrap()
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn finalizing_arb() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("anyone", &[]),
        ExecuteMsg::FinializeArb {},
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 2,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("bluna_hub"),
                msg: to_binary(&basset::hub::ExecuteMsg::WithdrawUnbonded {}).unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success
        }
    );
}

#[test]
fn replying_after_withdrawal() {
    let mut deps = setup_test();

    let response = SubMsgExecutionResponse {
        events: vec![Event::new("from_contract")
            .add_attribute("action", "finish_burn")
            .add_attribute("from", "bluna_hub")
            .add_attribute("amount", "105000000000")],
        data: None,
    };

    let res = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 2,
            result: ContractResult::Ok(response),
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("red_bank"),
                msg: to_binary(&mars_core::red_bank::msg::ExecuteMsg::RepayNative {
                    denom: String::from("uluna"),
                    on_behalf_of: None
                })
                .unwrap(),
                funds: vec![Coin::new(101_000_000_000, "uluna")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("alice"),
                amount: vec![Coin::new(800_000_000, "uluna")] // 400_000_000 * 0.2
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        res.messages[2],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("bob"),
                amount: vec![Coin::new(400_000_000, "uluna")] // 400_000_000 * 0.1
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("owner"),
                amount: vec![Coin::new(2_800_000_000, "uluna")] // the remainder after sending shares to Alice and Bob
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}
