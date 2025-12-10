use investment::{
    balance::ContractBalances,
    contract::{InvestmentContract, InvestmentContractClient},
    investment::{Investment, InvestmentStatus}
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        TokenAdminClient::new(e, &sac.address()),
    )
}

pub struct TestData<'a> {
    pub user: Address,
    pub project_address: Address,
    pub admin: Address,
    pub client: InvestmentContractClient<'a>,
    pub token: TokenClient<'a>,
    pub token_admin: TokenAdminClient<'a>,
}

pub fn create_investment_contract(
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

pub fn do_mint_and_invest(e: &Env, test_data: &TestData) {
    let another_user: Address = Address::generate(e);
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.token_admin.mint(&another_user, &1000000);

    test_data.client.invest(&test_data.user, &100000);
    test_data.client.invest(&another_user, &50000);
}

pub fn do_test_investment(e: &Env, test_data: TestData, investment_user: Investment, return_type: u32) {
    let mut last_transfer_ts: u64 = 0;
    let claimable_ts = investment_user.claimable_ts;
    let flows = [1_i128, 2_i128, 3_i128];
    let advance_secs = 30 * 24 * 60 * 61;

    let mut contract_balances: ContractBalances = test_data.client.get_contract_balance();
    let mut last_contract_payments_balance: i128 = contract_balances.payments;

    for multiplier in flows.iter() {
        last_transfer_ts = do_process_investor_payment_test(&test_data, &last_transfer_ts, *multiplier, InvestmentStatus::CashFlowing, return_type, claimable_ts);

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

pub fn do_process_investor_payment_test(
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
