#![cfg(test)]

use crate::{
    balance::{calculate_rate_denominator, ContractBalances},
    contract::{InvestmentContract, InvestmentContractClient},
    investment::{Investment, InvestmentStatus},
    multisig::MultisigStatus
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
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
    user: Address,
    project_address: Address,
    admin: Address,
    client: InvestmentContractClient<'a>,
    token: TokenClient<'a>,
    token_admin: TokenAdminClient<'a>,
}

fn create_investment_contract(
    e: &Env,
    i_rate: u32,
    claim_block_days: u64,
    goal: i128,
    return_type: u32,
    return_months: u32,
    min_per_investment: i128,
) -> TestData<'_> {
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let project_address = Address::generate(&e);
    let (token, token_admin) = create_token_contract(&e, &admin);

    let client = InvestmentContractClient::new(
        e,
        &e.register(
            InvestmentContract {},
            (
                admin.clone(),
                project_address.clone(),
                token.address.clone(),
                i_rate,
                claim_block_days,
                goal,
                return_type,
                return_months,
                min_per_investment,
            ),
        ),
    );

    TestData {
        user,
        project_address,
        admin,
        client,
        token,
        token_admin,
    }
}

#[test]
fn test_commision_calculator() {
    assert_eq!(calculate_rate_denominator(&90_i128), 10_u32);
    assert_eq!(calculate_rate_denominator(&120_i128), 10_u32);
    assert_eq!(calculate_rate_denominator(&150_i128), 10_u32);
    assert_eq!(calculate_rate_denominator(&500_i128), 11_u32);
    assert_eq!(calculate_rate_denominator(&1900_i128), 14_u32);
}

#[test]
fn test_investment_reverse_loan() {
    let e = Env::default();
    let test_data =create_investment_contract(&e, 500_u32, 7_u64, 0_i128, 1_u32, 4_u32, 100000_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.client.address, &300000);
    test_data.token_admin.mint(&test_data.project_address, &300000);
    test_data.token_admin.mint(&test_data.admin, &300000);

    let investment_user: Investment = test_data.client.invest(&test_data.user, &100000);

    let current_ts = e.ledger().timestamp();
    e.ledger().set_timestamp(current_ts + 604888);

    test_data.client.add_company_transfer(&100000_i128);

    do_test_investment(&e, test_data, investment_user, 1);
}

#[test]
fn test_investment_coupon() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 0_i128, 2_u32, 4_u32, 100000_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.client.address, &300000);
    test_data.token_admin.mint(&test_data.project_address, &300000);
    test_data.token_admin.mint(&test_data.admin, &300000);

    let investment_user: Investment = test_data.client.invest(&test_data.user, &100000);

    let current_ts = e.ledger().timestamp();
    e.ledger().set_timestamp(current_ts + 604888);

    test_data.client.add_company_transfer(&100000_i128);

    do_test_investment(&e, test_data, investment_user, 2);

}

#[test]
fn test_check_contract_balance_fails() {
    let e = Env::default();
    let test_data =create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);
    test_data.token_admin.mint(&test_data.project_address, &36000);

    e.ledger().set_timestamp(27 * 24 * 60 * 60);
    test_data.client.single_withdrawn(&140000_i128);
    assert!(test_data.client.check_reserve_balance() > 0);
}

#[test]
fn test_check_contract_balance() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);
    let contract_balances: ContractBalances = test_data.client.get_contract_balance();

    assert_eq!(
        test_data.token.balance(&test_data.client.address),
        150000_i128
    );
    assert!(contract_balances.comission > 0_i128);
    assert!(contract_balances.reserve > contract_balances.comission);
    assert!(contract_balances.project > contract_balances.reserve);

    e.ledger().set_timestamp(27 * 24 * 60 * 60);
    test_data.token_admin.mint(&test_data.admin, &100000_i128);
    test_data.client.add_company_transfer(&99000_i128);

    assert_eq!(test_data.client.check_reserve_balance(), 0_i128);
}

#[test]
fn test_multisig() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);

    assert_eq!(
        test_data.token.balance(&test_data.client.address),
        150000_i128
    );
    assert_eq!(
        test_data
            .client
            .multisig_withdrawn(&test_data.project_address, &40000),
        MultisigStatus::WaitingForSignatures
    );
    assert_eq!(
        test_data
            .client
            .multisig_withdrawn(&test_data.admin, &40000),
        MultisigStatus::Completed
    );
    assert_eq!(
        test_data.token.balance(&test_data.client.address),
        110000_i128
    );
    assert_eq!(
        test_data.token.balance(&test_data.project_address),
        40000_i128
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #21)")]
fn test_invalid_address_signing_multisig() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);

    test_data
        .client
        .multisig_withdrawn(&test_data.project_address, &40000);
    test_data.client.multisig_withdrawn(&test_data.user, &40000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_multisig_expired() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);

    test_data
        .client
        .multisig_withdrawn(&test_data.project_address, &40000);
    e.ledger().set_timestamp(86600);
    test_data
        .client
        .multisig_withdrawn(&test_data.admin, &40000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #23)")]
fn test_multisig_different_amount() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);

    do_mint_and_invest(&e, &test_data);

    test_data
        .client
        .multisig_withdrawn(&test_data.project_address, &40000);
    test_data
        .client
        .multisig_withdrawn(&test_data.admin, &45000);
}

#[test]
fn test_single_withdrawn() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);
    do_mint_and_invest(&e, &test_data);

    test_data.client.single_withdrawn(&40000_i128);
    assert_eq!(
        test_data.token.balance(&test_data.project_address),
        40000_i128
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_single_withdrawn_insufficient_balance() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);
    do_mint_and_invest(&e, &test_data);

    test_data.client.single_withdrawn(&160000_i128);
}

#[test]
fn test_add_company_transfer() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);
    do_mint_and_invest(&e, &test_data);

    test_data
        .token_admin
        .mint(&test_data.project_address, &1000000);
    test_data
        .token
        .transfer(&test_data.project_address, &test_data.admin, &1000000);
    test_data.client.add_company_transfer(&1000000);

    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert!(contract_balances.reserve > 1000000);
    assert_eq!(contract_balances.reserve_contributions, 1000000_i128);
}

#[test]
fn test_move_funds_to_reserve() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100000_i128);
    do_mint_and_invest(&e, &test_data);

    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    let project_balance = contract_balances.project;

    test_data.client.move_funds_to_the_reserve(&50000_i128);
    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert!(contract_balances.reserve > 50000);
    assert!(contract_balances.project <= project_balance - 50000);
}

fn do_mint_and_invest(e: &Env, test_data: &TestData) {
    let another_user: Address = Address::generate(e);
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&another_user, &1000000);

    test_data.client.invest(&test_data.user, &100000);
    test_data.client.invest(&another_user, &50000);
}

fn do_test_investment(e: &Env, test_data: TestData, investment_user: Investment, return_type: u32) {
    let mut last_transfer_ts: u64 = 0;
    let claimable_ts = investment_user.claimable_ts;
    let flows = [1_i128, 2_i128, 3_i128];
    let advance_secs = 30 * 24 * 60 * 61;

    let mut contract_balances: ContractBalances = test_data.client.get_contract_balance();
    let mut last_contract_payments_balance: i128 = contract_balances.payments;

    for multiplier in flows.iter() {
        // Procesamos el pago del inversor
        last_transfer_ts = do_process_investor_payment_test(&test_data, &last_transfer_ts, *multiplier, InvestmentStatus::CashFlowing,return_type, claimable_ts);

        e.ledger().set_timestamp(last_transfer_ts + advance_secs);
        contract_balances = test_data.client.get_contract_balance();
        assert!(contract_balances.payments > last_contract_payments_balance);
        last_contract_payments_balance = contract_balances.payments;
    }

    test_data.token.transfer(&test_data.project_address, &test_data.admin, &30000_i128);
    test_data.client.add_company_transfer(&30000_i128);
    do_process_investor_payment_test(
        &test_data,
        &last_transfer_ts,
        4_i128,
        InvestmentStatus::Finished,
        return_type,
        claimable_ts,
    );

    contract_balances = test_data.client.get_contract_balance();
    assert!(contract_balances.payments > last_contract_payments_balance);
}

fn do_process_investor_payment_test(
    test_data: &TestData,
    last_transfer_ts: &u64,
    multiplier: i128,
    status: InvestmentStatus,
    return_type: u32,
    claimable_ts: u64,
) -> u64 {
    let investment_user_1: Investment = test_data
        .client
        .process_investor_payment(&test_data.user, &claimable_ts);
    assert_eq!(investment_user_1.status, status);
    assert!(investment_user_1.last_transfer_ts > *last_transfer_ts);

    if return_type == 2 && status == InvestmentStatus::Finished {
        assert_eq!(
            investment_user_1.paid,
            ((investment_user_1.regular_payment * multiplier) + investment_user_1.deposited)
        );
    } else {
        assert_eq!(
            investment_user_1.paid,
            (investment_user_1.regular_payment * multiplier)
        );
    }

    investment_user_1.last_transfer_ts
}
