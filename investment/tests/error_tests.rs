mod common;

use common::{create_investment_contract, do_mint_and_invest};
use soroban_sdk::Env;

// ==================== Constructor Error Tests ====================

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_interest_rate_zero() {
    let e = Env::default();
    create_investment_contract(&e, 0_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_goal_zero() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 0_i128, 1_u32, 4_u32, 100_i128);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_invalid_return_type() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 0_u32, 4_u32, 100_i128);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_return_months_zero() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 0_u32, 100_i128);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_min_investment_zero() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 0_i128);
}

// ==================== Investment Error Tests ====================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_goal_reached() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 90000_i128, 2_u32, 4_u32, 100_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &89000);
    test_data.client.invest(&test_data.user, &2200);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_invest_insufficient_balance() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    // Mint less tokens than needed so balance is insufficient
    test_data.token_admin.mint(&test_data.user, &50000);
    test_data.client.invest(&test_data.user, &100000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_invest_amount_less_than_minimum() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    // Attempt to invest less than the minimum (min_per_investment = 100)
    test_data.client.invest(&test_data.user, &50);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #27)")]
fn test_invest_contract_paused() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    // Pause the contract
    test_data.client.stop_investments();

    // Attempt to invest with the contract paused
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &100000);
}

// ==================== Payment Processing Error Tests ====================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_process_payment_address_not_invested() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    // Try to process payment for an address that has not invested
    let ts = e.ledger().timestamp();
    test_data.client.process_investor_payment(&test_data.user, &ts);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #15)")]
fn test_process_payment_not_claimable_yet() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);

    // Try to process payment before claimable_ts (do not advance ledger time)
    test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn test_process_payment_investment_finished() {
    use investment::investment::InvestmentStatus;
    use soroban_sdk::testutils::Ledger;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.admin, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);

    // Advance to claimable_ts and process payments until investment is finished
    e.ledger().set_timestamp(investment.claimable_ts);
    test_data.client.add_company_transfer(&500000);

    // Process payments until the investment is finished
    let mut count = 0;
    let mut inv = test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
    while inv.status != InvestmentStatus::Finished && count < 4 {
        let current_ts = e.ledger().timestamp();
        e.ledger().set_timestamp(current_ts + (31 * 24 * 60 * 60)); // +1 month
        inv = test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
        count += 1;
    }

    // Attempt to process payment when the investment is already finished
    test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #17)")]
fn test_process_payment_next_transfer_not_ready() {
    use soroban_sdk::testutils::Ledger;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&test_data.admin, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);

    // Advance to claimable_ts
    e.ledger().set_timestamp(investment.claimable_ts);
    test_data.client.add_company_transfer(&500000);
    test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);

    // Advance only 15 days (less than a month)
    let current_ts = e.ledger().timestamp();
    e.ledger().set_timestamp(current_ts + (15 * 24 * 60 * 60));

    // Try to process payment before a month has passed since the last transfer
    test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
}

// ==================== Withdrawal Error Tests ====================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_single_withdrawn_insufficient_balance() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    do_mint_and_invest(&e, &test_data);
    test_data.client.single_withdrawn(&160000_i128);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #24)")]
fn test_move_funds_insufficient_project_balance() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    do_mint_and_invest(&e, &test_data);
    
    // Try outmoving more funds than available in project balance
    test_data.client.move_funds_to_the_reserve(&500000_i128);
}

// ==================== Transfer Error Tests ====================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_add_company_transfer_insufficient_balance() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    // Admin does not have tokens minted
    test_data.client.add_company_transfer(&100000_i128);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_process_payment_insufficient_reserve() {
    use soroban_sdk::testutils::Ledger;

    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.token_admin.mint(&test_data.user, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);
    
    // Advance to claimable_ts without adding funds to the reserve
    e.ledger().set_timestamp(investment.claimable_ts);
    
    // Attempt to process payment without sufficient funds in the reserve
    test_data.client.process_investor_payment(&test_data.user, &investment.claimable_ts);
}


// ==================== Lifecycle Error Tests ====================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #26)")]
fn test_stop_investments_already_paused() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.client.stop_investments();
    test_data.client.stop_investments();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #25)")]
fn test_restart_investments_already_active() {
    let e = Env::default();
    let test_data = create_investment_contract(&e, 500_u32, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128);
    
    test_data.client.restart_investments();
}

