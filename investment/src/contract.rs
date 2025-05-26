use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Map, String};

use crate::balance::{
    decrement_project_balance_from_raw_amount,
    decrement_project_balance_or_reserve_fund_from_raw_amount,
    increment_reserve_fund_from_raw_amount, recalculate_contract_balances_from_amount, Amount,
    CalculateAmounts, ContractBalances,
};
use crate::claim::{calculate_next_claim, Claim};
use crate::data::{ContractData, DataKey, Error, FromNumber, State};
use crate::investment::{
    build_investment, process_investment_payment, Investment, InvestmentReturnType,
    InvestmentStatus,
};
use crate::multisig::{MultisigRequest, MultisigStatus};
use crate::storage::{
    get_balances_or_new, get_claims_map_or_new, get_contract_data, get_investment,
    get_multisig_or_new, set_investment, set_multisig, update_claims_map,
    update_contract_balances, update_contract_data,
};

macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

fn get_token(env: &Env) -> TokenClient {
    let contract_data = get_contract_data(&env);
    let tk = token::Client::new(&env, &contract_data.token);
    tk
}

fn update_investment(e: &Env, addr: Address, investment: &Investment) {
    set_investment(e, addr.clone(), investment);
    let mut claims_map: Map<Address, Claim> = get_claims_map_or_new(e);

    claims_map.set(addr.clone(), calculate_next_claim(e, investment));
    update_claims_map(e, claims_map);
}

#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {

    /// Constructor for the contract.
    ///
    /// Initializes the contract with the required parameters.
    /// Ensures that the interest rate (i_rate) is not zero and the return type is supported.
    /// Also requires authentication from the admin address.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment provided by Soroban.
    /// * `admin_addr` - The contract administrator's address.
    /// * `project_address` - The project address to which the funding is directed.
    /// * `token_addr` - The token address used for transactions.
    /// * `i_rate` - The interest rate; must be greater than zero.
    /// * `claim_block_days` - The number of days to block claims for investments.
    /// * `goal` - The funding goal (can be 0 if there is no goal).
    /// * `return_type` - The investment return type expressed as a number.
    /// * `return_months` - The number of months over which the return is calculated.
    /// * `min_per_investment` - The minimum amount allowed per investment.
    ///
    /// # Panics
    ///
    /// * Panics if:
    ///   - The interest rate is 0.
    ///   - The provided `return_type` is not supported.
    pub fn __constructor(
        env: Env,
        admin_addr: Address,
        project_address: Address,
        token_addr: Address,
        i_rate: u32,
        claim_block_days: u64,
        goal: i128,
        return_type: u32,
        return_months: u32,
        min_per_investment: i128,
    ) {
        if i_rate == 0 {
            panic!("Interest rate cannot be 0");
        }

        admin_addr.require_auth();
        if let Some(ret_type) = InvestmentReturnType::from_number(return_type) {
            let contract_data = ContractData {
                interest_rate: i_rate,
                claim_block_days,
                token: token_addr,
                project_address,
                admin: admin_addr,
                state: State::Initialized,
                return_type: ret_type,
                return_months,
                min_per_investment,
                goal,
            };

            update_contract_data(&env, &contract_data);
        } else {
            panic!("unsupported return type");
        }
    }

    /// Processes an investor's payment.
    ///
    /// Verifies that the investor has an active investment and that the payment is claimable based on time.
    /// Updates the investment, adjusts the contract balances, and transfers the processed amount to the investor.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The address of the investor receiving the payment.
    ///
    /// # Returns
    ///
    /// * On success, returns an updated `Investment` object.
    /// * On failure, returns an error of type `Error`.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if the address does not have an investment.
    /// * `AddressInvestmentIsNotClaimableYet` if it is not yet time to claim the payment.
    /// * `AddressInvestmentIsFinished` if the investment has already been finished.
    /// * `ContractInsufficientBalance` if the contract does not have sufficient funds.
    /// * `AddressInvestmentNextTransferNotClaimableYet` if the next transfer is not claimable yet.
    pub fn process_investor_payment(env: Env, addr: Address, ts: u64) -> Result<Investment, Error> {

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let mut investment: Investment;
        match get_investment(&env, addr.clone(), ts) {
            Some(inv) => investment = inv,
            None => return Err(Error::AddressHasNotInvested),
        }
        require!(
            env.ledger().timestamp() >= investment.claimable_ts,
            Error::AddressInvestmentIsNotClaimableYet
        );
        require!(
            investment.status != InvestmentStatus::Finished,
            Error::AddressInvestmentIsFinished
        );

        let mut contract_balances: ContractBalances = get_balances_or_new(&env);

        let seconds_in_a_month = 30 * 24 * 60 * 60;
        if investment.last_transfer_ts == 0 || (env.ledger().timestamp() - investment.last_transfer_ts) > seconds_in_a_month
        {
            let tk = get_token(&env);
            let amount_to_transfer: i128 = process_investment_payment(&env, &mut investment, &contract_data);

            require!(
                amount_to_transfer < (contract_balances.project + contract_balances.reserve_fund),
                Error::ContractInsufficientBalance
            );

            tk.transfer(&env.current_contract_address(), &addr, &amount_to_transfer);

            update_investment(&env, addr.clone(), &investment);
            decrement_project_balance_or_reserve_fund_from_raw_amount(&mut contract_balances, &amount_to_transfer);
            update_contract_balances(&env, &contract_balances);

            return Ok(investment);
        } else {
            return Err(Error::AddressInvestmentNextTransferNotClaimableYet);
        }
    }

    /// Allows an investor to make an investment.
    ///
    /// Verifies that the investment amount is positive, that the contract is still open for investments,
    /// that the address does not already have an active investment, and that the address has sufficient funds.
    /// Transfers the investment amount and updates the contract balances accordingly.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address.
    /// * `amount` - The investment amount.
    ///
    /// # Returns
    ///
    /// * Returns an `Investment` object representing the registered investment on success.
    /// * Returns an error of type `Error` on failure.
    ///
    /// # Errors
    ///
    /// * `AmountLessOrEqualThan0` if the investment amount is less than or equal to 0.
    /// * `ContractFinancingReached` if the contract has reached its financing goal.
    /// * `AddressAlreadyInvested` if the address has already made an investment.
    /// * `ContractHasReachedInvestmentGoal` if the contract has already reached the goal (applicable if `goal` is not 0).
    /// * `AddressInsufficientBalance` if the address does not have sufficient balance.
    pub fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error> {
        require!(amount > 0, Error::AmountLessOrEqualThan0);

        let contract_data: ContractData = get_contract_data(&env);
        require!(contract_data.state != State::FinancingReached, Error::ContractFinancingReached);
      //  require!(!has_investment(&env, addr.clone()), Error::AddressAlreadyInvested);

        addr.require_auth();
        let tk = get_token(&env);

        require!(contract_data.goal == 0 || tk.balance(&env.current_contract_address()) < contract_data.goal, Error::ContractHasReachedInvestmentGoal);
        require!(tk.balance(&addr) >= amount, Error::AddressInsufficientBalance);

        let amounts: Amount = Amount::from_investment(&amount, &contract_data.interest_rate);
        tk.transfer(&addr, &env.current_contract_address(), &amount);

        let mut contract_balances = get_balances_or_new(&env);
        recalculate_contract_balances_from_amount(&mut contract_balances, &amounts);
        update_contract_balances(&env, &contract_balances);

        let addr_investment: Investment = build_investment(&env, &contract_data, &amount);
        update_investment(&env, addr.clone(), &addr_investment);

        Ok(addr_investment)
    }

    /// Retrieves the current balances of the contract.
    ///
    /// Requires authentication from the admin.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * Returns a `ContractBalances` object representing the contract's funds,
    ///   or an error if something goes wrong.
    pub fn get_contract_balance(env: Env) -> Result<ContractBalances, Error> {
        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let contract_balances: ContractBalances = get_balances_or_new(&env);

        Ok(contract_balances)
    }

    /// Stops accepting new investments.
    ///
    /// Allows the admin to change the contract state to 'FinancingReached', which prevents new investments.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * Returns `true` on success, or an error if something goes wrong.
    pub fn stop_investments(env: Env) -> Result<bool, Error> {
        let mut contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();
        contract_data.state = State::FinancingReached;
        update_contract_data(&env, &contract_data);

        Ok(true)
    }

    /// Processes a multisig withdrawal request.
    ///
    /// Verifies the validity of the signature, that the requested amount is correct,
    /// and that the multisig request has not expired. If all required signatures have been collected,
    /// the amount is transferred from the contract to the project's address and the multisig request is removed.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The address signing the withdrawal request.
    /// * `amount` - The amount requested for withdrawal.
    ///
    /// # Returns
    ///
    /// * Returns a `MultisigStatus` indicating whether the request has been completed or if it is still waiting for signatures.
    ///
    /// # Errors
    ///
    /// * `ContractInsufficientBalance` if the contract does not have sufficient funds.
    /// * `WithdrawalUnexpectedSignature` if the signature is not valid for this request.
    /// * `WithdrawalExpiredSignature` if the multisig request has expired.
    /// * `WithdrawalInvalidAmount` if the requested amount does not match the multisig amount.
    pub fn multisig_withdrawn(env: Env, addr: Address, amount: i128) -> Result<MultisigStatus, Error> {
        let valid_ts = env.ledger().timestamp() + 86400;

        let tk = get_token(&env);
        let contract_balances: ContractBalances = get_balances_or_new(&env);

        require!(contract_balances.project > amount, Error::ContractInsufficientBalance);

        let contract_data: ContractData = get_contract_data(&env);
        let multisig_claim: String = String::from_str(&env, "project_withdrawn");
        let mut multisig: MultisigRequest = get_multisig_or_new(
            &env,
            &contract_data,
            multisig_claim,
            2_u32,
            amount,
            valid_ts,
        );

        require!(multisig.is_valid_signature(addr.clone()), Error::WithdrawalUnexpectedSignature);
        require!(!multisig.is_expired(env.ledger().timestamp()), Error::WithdrawalExpiredSignature);
        require!(amount == multisig.amount, Error::WithdrawalInvalidAmount);

        addr.require_auth();
        multisig.add_sig(addr);

        if multisig.is_completed() {
            env.storage().temporary().remove(&DataKey::MultisigRequest);
            tk.transfer(
                &env.current_contract_address(),
                &contract_data.project_address,
                &multisig.amount,
            );
            return Ok(MultisigStatus::Completed);
        }

        set_multisig(&env, &multisig);
        Ok(MultisigStatus::WaitingForSignatures)
    }

    /// Allows the admin to perform a single withdrawal.
    ///
    /// Requires admin authentication and sufficient funds in the project balance.
    /// Transfers the specified amount and updates the internal balances accordingly.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to withdraw.
    ///
    /// # Returns
    ///
    /// * Returns `true` on success, or an error if something fails.
    ///
    /// # Errors
    ///
    /// * `ContractInsufficientBalance` if the contract does not have enough funds.
    pub fn single_withdrawn(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let mut contract_balances: ContractBalances = get_balances_or_new(&env);
        require!(
            contract_balances.project > amount,
            Error::ContractInsufficientBalance
        );

        let tk = get_token(&env);
        tk.transfer(
            &env.current_contract_address(),
            &contract_data.project_address,
            &amount,
        );
        decrement_project_balance_from_raw_amount(&mut contract_balances, &amount);
        update_contract_balances(&env, &contract_balances);

        Ok(true)
    }

    /// Checks the project address balance and determines if additional funds are required to cover pending claims.
    ///
    /// Requires authentication from the admin and reviews the claims map to calculate the minimum funds needed.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * Returns the additional amount required if the current project balance is insufficient,
    ///   otherwise returns 0.
    pub fn check_project_address_balance(env: Env) -> Result<i128, Error> {
        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let claims_map: Map<Address, Claim> = get_claims_map_or_new(&env);
        let project_balances: ContractBalances = get_balances_or_new(&env);
        let mut min_funds: i128 = 0;

        for (_addr, next_claim) in claims_map.iter() {
            if next_claim.is_claim_next(&env) {
                min_funds += next_claim.amount_to_pay;
            }
        }

        if min_funds > 0 {
            if project_balances.project < min_funds {
                let diff_to_contribute: i128 = min_funds - project_balances.project;
                return Ok(diff_to_contribute);
            }
        }

        Ok(0_i128)
    }

    /// Allows a company to add a transfer to the contract.
    ///
    /// Transfers an amount from the admin address (which previously had been received an amount from company) to the contract and updates the reserve fund balance.
    ///
    /// # Parameters
    ///
    /// * `e` - The execution environment.
    /// * `amount` - The amount to transfer.
    ///
    /// # Returns
    ///
    /// * Returns `true` on success, or an error if something goes wrong.
    ///
    /// # Errors
    ///
    /// * Requires authentication of the company address.
    pub fn add_company_transfer(e: Env, amount: i128) -> Result<bool, Error> {
        let contract_data: ContractData = get_contract_data(&e);
        contract_data.admin.require_auth();

        let tk = get_token(&e);
        tk.transfer(&contract_data.admin, &e.current_contract_address(), &amount);

        let mut contract_balances = get_balances_or_new(&e);
        increment_reserve_fund_from_raw_amount(&mut contract_balances, &amount);
        update_contract_balances(&e, &contract_balances);

        Ok(true)
    }
}
