#![cfg(test)]

use crate::contract::{InvestmentContract, InvestmentContractClient};
use crate::data::{ContractBalances, Investment, InvestmentStatus};
use soroban_sdk::{Env, testutils::{Address as _, Ledger}, Address, testutils::Logs, token};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

extern crate std;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        TokenAdminClient::new(e, &sac.address()),
    )
}

struct TestData<'a> {
    admin: Address,
    user: Address,
    project_address: Address,
    client:  InvestmentContractClient<'a>,
    token: TokenClient<'a>,
    token_admin: TokenAdminClient<'a>
}

fn init_test_data(e: &Env) -> TestData {
    e.mock_all_auths();

    let contract_id = e.register( InvestmentContract, {});
    let client = InvestmentContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let project_address = Address::generate(&e);
    let (token, token_admin) = create_token_contract(&e, &admin);

    TestData {
        admin,
        user,
        project_address,
        client,
        token,
        token_admin
    }
}

#[test]
fn test_init() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    assert_eq!(test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &500_u32, 
        &30_u64,
        &1000_i128,
        &1_u32,
        &24_u32,
        &100_i128), 
    true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_init_fail_invalid_return_type() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &500_u32, 
        &30_u64,
        &1000_i128,
        &4_u32,
        &4_u32,
        &100000_i128
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_init_fail_i_rate_is_0() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &0_u32, 
        &30_u64,
        &1000_i128,
        &4_u32,
        &4_u32,
        &100000_i128
    );
}

#[test]
fn test_investment() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &500_u32, 
        &7_u64,
        &0_i128,
        &1_u32,
        &4_u32,
        &100000_i128
    );

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.client.address, &3000);

    let investment_user_1: Investment = test_data.client.invest(&test_data.user, &100000);
    assert_eq!(investment_user_1.status, InvestmentStatus::Blocked);
    assert_eq!(investment_user_1.deposited, (5000 + 93000));
    assert_eq!(investment_user_1.accumulated_interests, 4900);
    assert_eq!(investment_user_1.total, (98000 + 4900));
    assert_eq!(investment_user_1.regular_payment, 25725);

    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert_eq!(test_data.token.balance(&test_data.client.address), 103000_i128);
    assert_eq!(contract_balances.comission, 2000_i128);
    assert_eq!(contract_balances.reserve_fund, 5000_i128);
    assert_eq!(contract_balances.project, 93000_i128);

    let current_ts = e.ledger().timestamp();
    e.ledger().set_timestamp(current_ts + 604888);

    let mut last_transfer_ts: u64 = 0;

    test_data.client.get_contract_balance();
    last_transfer_ts = do_claim_test( &test_data, &last_transfer_ts, 1_i128, InvestmentStatus::CashFlowing);
    e.ledger().set_timestamp(last_transfer_ts + (30 * 24 * 60 * 61));

    test_data.client.get_contract_balance();
    last_transfer_ts = do_claim_test(&test_data, &last_transfer_ts,  2_i128, InvestmentStatus::CashFlowing);
    e.ledger().set_timestamp(last_transfer_ts + (30 * 24 * 60 * 61));

    test_data.client.get_contract_balance();
    last_transfer_ts = do_claim_test(&test_data, &last_transfer_ts,  3_i128, InvestmentStatus::CashFlowing);
    e.ledger().set_timestamp(last_transfer_ts + (30 * 24 * 60 * 61));

    test_data.client.get_contract_balance();
    do_claim_test(&test_data, &last_transfer_ts,  4_i128, InvestmentStatus::Finished);

    assert!(test_data.token.balance(&test_data.client.address) < investment_user_1.regular_payment);

    let logs = e.logs().all();
    std::println!("{}", logs.join("\n"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #20)")]
fn test_check_contract_balance_fails() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &500_u32, 
        &7_u64,
        &1000000_i128,
        &1_u32,
        &4_u32,
        &100000_i128
    );

    let another_user: Address = Address::generate(&e);
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&another_user, &1000000);
    test_data.token_admin.mint(&test_data.project_address, &36000);

    test_data.client.invest(&test_data.user, &100000);
    test_data.client.invest(&another_user, &50000);
    
    test_data.token.transfer(&test_data.client.address, &test_data.project_address,&140000);

    e.ledger().set_timestamp(27 * 24 * 60 * 60);
    test_data.client.check_project_address_balance();

    let logs = e.logs().all();
    std::println!("{}", logs.join("\n"));

}

#[test]
fn test_check_contract_balance() {
    let e = Env::default();
    let test_data = init_test_data(&e);
    test_data.client.init(
        &test_data.admin, 
        &test_data.project_address,
        &test_data.token.address, 
        &500_u32, 
        &7_u64,
        &1000000_i128,
        &1_u32,
        &4_u32,
        &100000_i128
    );

    let another_user: Address = Address::generate(&e);
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&another_user, &1000000);

    test_data.client.invest(&test_data.user, &100000);
    test_data.client.invest(&another_user, &50000);
    let contract_balances: ContractBalances = test_data.client.get_contract_balance();

    assert_eq!(test_data.token.balance(&test_data.client.address), 150000_i128);
    assert_eq!(contract_balances.comission, 3000_i128);
    assert_eq!(contract_balances.reserve_fund, 7500_i128);
    assert_eq!(contract_balances.project, 139500_i128);

    e.ledger().set_timestamp(27 * 24 * 60 * 60);
    test_data.token.transfer(&test_data.client.address, &test_data.project_address,&40000);
    assert_eq!(test_data.client.check_project_address_balance(), 99500_i128);

}

fn do_claim_test(test_data: &TestData, last_transfer_ts: &u64, multiplier: i128, status: InvestmentStatus) -> u64  {
    let investment_user_1: Investment = test_data.client.claim(&test_data.user);
    assert_eq!(investment_user_1.status, status);
    assert!(investment_user_1.last_transfer_ts > *last_transfer_ts );
    assert_eq!(investment_user_1.paid, (investment_user_1.regular_payment * multiplier));

    let current_contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert_eq!(current_contract_balances.comission, 2000_i128);
    assert_eq!(current_contract_balances.reserve_fund, 5000_i128);
    assert!(test_data.token.balance(&test_data.client.address) > current_contract_balances.sum());
    
    investment_user_1.last_transfer_ts
}