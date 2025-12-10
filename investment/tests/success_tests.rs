mod common;

use common::{create_investment_contract, do_mint_and_invest, do_test_investment};
use investment::balance::{calculate_rate_denominator, ContractBalances};
use investment::investment::Investment;
use soroban_sdk::{testutils::Ledger, Env};

#[test]
fn test_commision_calculator() {
    assert_eq!(calculate_rate_denominator(&(90_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(120_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(150_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(500_i128 * 10_000_000), 7), 11_u32);
    assert_eq!(calculate_rate_denominator(&(1900_i128 * 10_000_000), 7), 14_u32);
}

#[test]
fn test_investment_reverse_loan() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

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
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 2_u32, 4_u32, 100_i128);

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
fn test_check_contract_balance() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    do_mint_and_invest(&e, &test_data);
    let contract_balances: ContractBalances = test_data.client.get_contract_balance();

    assert_eq!(
        test_data.token.balance(&test_data.client.address),
        150000_i128
    );
    assert!(contract_balances.comission > 0_i128);
    assert!(contract_balances.reserve > contract_balances.comission);
    assert!(contract_balances.project > contract_balances.reserve);
}

#[test]
fn test_single_withdrawn() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    do_mint_and_invest(&e, &test_data);

    test_data.client.single_withdrawn(&40000_i128);
    assert_eq!(
        test_data.token.balance(&test_data.project_address),
        40000_i128
    );
}

#[test]
fn test_add_company_transfer() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    do_mint_and_invest(&e, &test_data);

    test_data.token_admin.mint(&test_data.project_address, &1000000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &1000000);
    test_data.client.add_company_transfer(&1000000);

    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert!(contract_balances.reserve > 1000000);
    assert_eq!(contract_balances.reserve_contributions, 1000000_i128);
}

#[test]
fn test_move_funds_to_reserve() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    do_mint_and_invest(&e, &test_data);

    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    let project_balance = contract_balances.project;

    test_data.client.move_funds_to_the_reserve(&50000_i128);
    let contract_balances: ContractBalances = test_data.client.get_contract_balance();
    assert!(contract_balances.reserve > 50000);
    assert!(contract_balances.project <= project_balance - 50000);
}

#[test]
fn test_stop_investments() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    let result = test_data.client.stop_investments();
    assert!(result);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    let invest_result = test_data.client.try_invest(&test_data.user, &100000);
    assert!(invest_result.is_err());
}

#[test]
fn test_restart_investments() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.client.stop_investments();
    let result = test_data.client.restart_investments();
    assert!(result);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);
    assert!(investment.deposited > 0);
}

#[test]
fn test_multiple_investments_same_user() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.admin, &600000);
    
    let investment_1 = test_data.client.invest(&test_data.user, &100000);
    let claimable_ts_1 = investment_1.claimable_ts;
    let deposited_1 = investment_1.deposited;
    
    let current_ts = e.ledger().timestamp();
    e.ledger().set_timestamp(current_ts + (8 * 24 * 60 * 60));
    
    let investment_2 = test_data.client.invest(&test_data.user, &50000);
    let claimable_ts_2 = investment_2.claimable_ts;
    let deposited_2 = investment_2.deposited;
    
    assert_ne!(claimable_ts_1, claimable_ts_2);
    assert!(deposited_1 > 0);
    assert!(deposited_2 > 0);
    
    let contract_balances = test_data.client.get_contract_balance();
    assert!(contract_balances.received_so_far >= deposited_1 + deposited_2);
    
    e.ledger().set_timestamp(claimable_ts_1);
    test_data.token_admin.mint(&test_data.client.address, &500000);
    test_data.client.add_company_transfer(&500000);
    
    let payment_1 = test_data.client.process_investor_payment(&test_data.user, &claimable_ts_1);
    assert!(payment_1.paid > 0);
    
    e.ledger().set_timestamp(claimable_ts_2);
    let payment_2 = test_data.client.process_investor_payment(&test_data.user, &claimable_ts_2);
    assert!(payment_2.paid > 0);
}

#[test]
fn test_invest_at_goal_limit() {
    let e = Env::default();
    let goal = 200000_i128;
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, goal, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);

    test_data.client.invest(&test_data.user, &100000);
    test_data.client.invest(&test_data.user, &50000);
    test_data.client.invest(&test_data.user, &40000);
    
    let contract_balances = test_data.client.get_contract_balance();
    
    assert!(contract_balances.received_so_far > 0);
    assert!(contract_balances.received_so_far <= goal);
    
    // Si ya alcanzamos el goal, verificar que no se puede invertir más
    if contract_balances.received_so_far >= goal {
        let result = test_data.client.try_invest(&test_data.user, &1000);
        assert!(result.is_err(), "Should not allow investment after reaching goal");
    }
}

#[test]
fn test_get_contract_balance_empty() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    let contract_balances = test_data.client.get_contract_balance();
    
    assert_eq!(contract_balances.project, 0_i128);
    assert_eq!(contract_balances.reserve, 0_i128);
    assert_eq!(contract_balances.comission, 0_i128);
    assert_eq!(contract_balances.payments, 0_i128);
    assert_eq!(contract_balances.received_so_far, 0_i128);
    assert_eq!(contract_balances.reserve_contributions, 0_i128);
}

#[test]
fn test_check_reserve_balance_no_investments() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    // No investments, should need 0 additional funds
    let needed = test_data.client.check_reserve_balance();
    assert_eq!(needed, 0_i128);
}

#[test]
fn test_check_reserve_balance_no_claims_in_next_week() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    let _investment = test_data.client.invest(&test_data.user, &100000);
    
    // Don't advance time - claimable_ts is far in the future (7 days + more)
    // The claim won't be within the next week
    let needed = test_data.client.check_reserve_balance();
    assert_eq!(needed, 0_i128, "No claims should be within next week");
}

#[test]
fn test_check_reserve_balance_claim_in_next_week_sufficient() {
    use soroban_sdk::testutils::Ledger;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.admin, &1000000);
    
    // Get timestamp when investment is created
    let invest_timestamp = e.ledger().timestamp();
    let _investment = test_data.client.invest(&test_data.user, &100000);
    
    // next_transfer_ts = invest_timestamp + SECONDS_IN_MONTH (30 days)
    // Advance time to 29 days and 18 hours (within next week window from the payment date)
    let seconds_in_month = 30 * 24 * 60 * 60_u64;
    e.ledger().set_timestamp(invest_timestamp + seconds_in_month - (6 * 60 * 60));
    
    // Add sufficient funds to reserve
    test_data.client.add_company_transfer(&500000);
    
    // Should need 0 additional funds (reserve is sufficient)
    let needed = test_data.client.check_reserve_balance();
    assert_eq!(needed, 0_i128, "Reserve should be sufficient");
}

#[test]
fn test_check_reserve_balance_claim_in_next_week_insufficient() {
    use soroban_sdk::testutils::Ledger;
    use investment::balance::ContractBalances;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    
    // Get timestamp when investment is created
    let invest_timestamp = e.ledger().timestamp();
    let investment = test_data.client.invest(&test_data.user, &100000);
    
    // next_transfer_ts = invest_timestamp + SECONDS_IN_MONTH (30 days)
    // Advance time to 27 days (3 days before next payment, within next week window)
    let seconds_in_month = 30 * 24 * 60 * 60_u64;
    e.ledger().set_timestamp(invest_timestamp + seconds_in_month - (3 * 24 * 60 * 60));
    
    // Don't add funds to reserve - it will be insufficient
    let balances: ContractBalances = test_data.client.get_contract_balance();
    let current_reserve = balances.reserve;
    let regular_payment = investment.regular_payment;
    
    // Should need the difference between regular_payment and current reserve
    let needed = test_data.client.check_reserve_balance();
    let expected_diff = regular_payment - current_reserve;
    assert_eq!(needed, expected_diff, "Should return exact difference needed");
    assert!(needed > 0, "Should need additional funds");
}

#[test]
fn test_check_reserve_balance_multiple_claims_in_next_week() {
    use soroban_sdk::testutils::{Address as _, Ledger};
    use investment::balance::ContractBalances;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    let user2 = soroban_sdk::Address::generate(&e);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&user2, &1000000);
    
    // Get timestamp when investments are created
    let invest_timestamp = e.ledger().timestamp();
    
    // Both users invest
    let investment1 = test_data.client.invest(&test_data.user, &100000);
    let investment2 = test_data.client.invest(&user2, &50000);
    
    // Both next_transfer_ts will be invest_timestamp + SECONDS_IN_MONTH
    // Advance time to 28 days (2 days before next payment, within next week window)
    let seconds_in_month = 30 * 24 * 60 * 60_u64;
    e.ledger().set_timestamp(invest_timestamp + seconds_in_month - (2 * 24 * 60 * 60));
    
    // Get current reserve
    let balances: ContractBalances = test_data.client.get_contract_balance();
    let current_reserve = balances.reserve;
    
    // Calculate total needed for both claims
    let total_needed = investment1.regular_payment + investment2.regular_payment;
    
    let needed = test_data.client.check_reserve_balance();
    let expected_diff = total_needed - current_reserve;
    
    assert_eq!(needed, expected_diff, "Should sum both claims and subtract reserve");
    assert!(needed > 0, "Should need additional funds for multiple claims");
}
