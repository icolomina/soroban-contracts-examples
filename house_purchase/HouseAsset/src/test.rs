#![cfg(test)]

use super::{Asset, AssetClient};
use soroban_sdk::{Env, Address, String, testutils::{Address as _}};

#[test]
fn initialize() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");
    assert_eq!(client.initialize(&owner, &asset_id), true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn already_initialized() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");
    client.initialize(&owner, &asset_id);
    client.initialize(&owner, &asset_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn non_initialized() {

    let env = Env::default();
    let client = create_client(&env);    
    let allowed_addr = Address::generate(&env);

    client.approve(&allowed_addr, &86400_u64);
}

#[test]
fn transfer() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");

    client.initialize(&owner, &asset_id);
    client.transfer(&new_owner);

    assert_eq!(client.owner(), new_owner);
}

#[test]
fn transfer_from() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let allowed_addr = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");

    client.initialize(&owner, &asset_id);
    client.approve(&allowed_addr, &86400_u64);
    client.transfer_from(&allowed_addr, &new_owner);

    assert_eq!(client.owner(), new_owner);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn transfer_from_address_not_approved() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let allowed_addr = Address::generate(&env);
    let not_allowed_addr = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");

    client.initialize(&owner, &asset_id);
    client.approve(&allowed_addr, &86400_u64);
    client.transfer_from(&not_allowed_addr, &new_owner);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn transfer_from_non_approvals() {
    let env = Env::default();
    let client = create_client(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let not_allowed_addr = Address::generate(&env);
    let asset_id = String::from_str(&env, "399fg7u6h69965h6");

    client.initialize(&owner, &asset_id);
    client.transfer_from(&not_allowed_addr, &new_owner);
}

fn create_client(env: &Env) -> AssetClient<'_> {
    env.mock_all_auths();
    let contract_id = env.register(Asset, () );
    let client = AssetClient::new(&env, &contract_id);

    client
}