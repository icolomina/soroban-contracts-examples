#![cfg(test)]

mod asset {
    soroban_sdk::contractimport!(
        file = "../HouseAsset/target/wasm32-unknown-unknown/release/house_asset.wasm"
    );
}

use super::{ HousePurchaseContract, HousePurchaseContractClient};
use soroban_sdk::{Env, testutils::Address as _, Address, token, String};
use token::Client as TokenClient;
use asset::Client as AssetClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        TokenAdminClient::new(e, &sac.address()),
    )
}

fn create_asset(e: &Env) -> AssetClient<'_> {
    let asset = AssetClient::new(e, &e.register(asset::WASM, ()));
    asset
}

struct TestData<'a> {
    buyer: Address,
    asset_contract: AssetClient<'a>,
    client:  HousePurchaseContractClient<'a>,
    sac_token: TokenClient<'a>
}

fn init_test_data(env: &Env) -> TestData<'_> {
    env.mock_all_auths();

    let client = HousePurchaseContractClient::new(
        env,
        &env.register(HousePurchaseContract, ())
    );

    let buyer: Address = Address::generate(&env);
    let owner: Address = Address::generate(&env);
    let asset_contract = create_asset(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");
    asset_contract.initialize(&owner, &asset_id);
    let token_admin = Address::generate(&env);

    let (sac_token, sac_token_admin) = create_token_contract(&env, &token_admin);
    sac_token_admin.mint(&buyer, &50000);

    TestData {
        buyer,
        asset_contract,
        client,
        sac_token
    }
}

#[test]
fn test_initialize() {
    let env = Env::default();
    let test_data = init_test_data(&env);

    assert_eq!(test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128), true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_already_initialized() {
    let env = Env::default();
    let test_data = init_test_data(&env);

    test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128);
    test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128);
}

#[test]
fn test_transfer() {
    let env = Env::default();
    let test_data = init_test_data(&env);

    test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128);
    test_data.client.transfer_first_payment();
    assert_eq!(test_data.sac_token.balance(&test_data.asset_contract.owner()), 5000);

    test_data.client.transfer_rest_of_payment();
    assert_eq!(test_data.sac_token.balance(&test_data.asset_contract.owner()), 45000);

    test_data.client.change_owner();
    assert_eq!(test_data.asset_contract.owner(), test_data.buyer);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_first_payment_contract_not_initialized() {
    let env = Env::default();
    let test_data = init_test_data(&env);
    test_data.client.transfer_first_payment();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_first_payment_not_transferred() {
    let env = Env::default();
    let test_data = init_test_data(&env);
    test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128);
    test_data.client.transfer_rest_of_payment();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_change_owner_without_payment_transferred() {
    let env = Env::default();
    let test_data = init_test_data(&env);
    test_data.client.initialize(&test_data.asset_contract.address, &test_data.buyer, &test_data.sac_token.address, &5000_i128, &45000_i128);
    test_data.client.transfer_first_payment();
    test_data.client.change_owner();
}