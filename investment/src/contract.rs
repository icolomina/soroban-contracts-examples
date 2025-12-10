use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Map};

use crate::constants::{SECONDS_IN_MONTH};
use crate::balance::{
    decrement_project_balance_from_company_withdrawal,
    decrement_project_balance_from_payment_to_investor,
    increment_reserve_balance_from_company_contribution,
    move_from_project_balance_to_reserve_balance, recalculate_contract_balances_from_investment,
    Amount, CalculateAmounts, ContractBalances,
};
use crate::claim::{calculate_next_claim, Claim};
use crate::data::{
    ContractData, Error, FromNumber, State, TOPIC_CONTRACT_BALANCE_UPDATED, TOPIC_CONTRACT_STATUS_UPDATED,
};
use crate::investment::{
    build_investment, process_investment_payment, Investment, InvestmentReturnType,
    InvestmentStatus,
};
use crate::storage::{
    get_balances_or_new, get_claims_map_or_new, get_contract_data, get_investment,
    set_investment, update_claims_map, update_contract_balances,
    update_contract_data,
};

macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
    ($($cond:expr, $err:expr),+) => {
        $(
            if !$cond {
                return Err($err);
            }
        )+
    };
}

fn get_token<'a>(env: &'a Env, contract_data: &ContractData) -> TokenClient<'a> {
    token::Client::new(env, &contract_data.token)
}

fn require_admin(env: &Env) -> ContractData {
    let contract_data = get_contract_data(env);
    contract_data.admin.require_auth();
    contract_data
}

fn update_investment(e: &Env, addr: &Address, investment: &Investment) {
    set_investment(e, addr, investment);
    let mut claims_map: Map<Address, Claim> = get_claims_map_or_new(e);

    claims_map.set(addr.clone(), calculate_next_claim(e, investment));
    update_claims_map(e, claims_map);
}

#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {
    /// Initializes the investment contract with configuration parameters.
    ///
    /// Sets up the contract with admin authentication, token configuration, investment rules,
    /// and return structure. The contract starts in 'Active' state.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment provided by Soroban.
    /// * `admin_addr` - The contract administrator's address (requires authentication).
    /// * `project_address` - The address that will receive withdrawn project funds.
    /// * `token_addr` - The token contract address used for all transactions.
    /// * `i_rate` - The interest rate percentage (must be > 0).
    /// * `claim_block_days` - Days investors must wait before claiming returns.
    /// * `goal` - The total funding goal (must be > 0).
    /// * `return_type` - The return model: 1=ReverseLoan, 2=Coupon.
    /// * `return_months` - Number of months for return payments (must be > 0).
    /// * `min_per_investment` - Minimum investment amount (must be > 0).
    ///
    /// # Errors
    ///
    /// * `InterestRateMustBeGreaterThanZero` if i_rate is 0.
    /// * `GoalMustBeGreaterThanZero` if goal is 0.
    /// * `ReturnMonthsMustBeGreaterThanZero` if return_months is 0.
    /// * `MinPerInvestmentMustBeGreaterThanZero` if min_per_investment is 0.
    /// * `UnsupportedReturnType` if return_type is not 1 or 2.
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
    ) -> Result<(), Error> {
        admin_addr.require_auth();

        require!(
            i_rate > 0, Error::InterestRateMustBeGreaterThanZero,
            goal > 0, Error::GoalMustBeGreaterThanZero,
            return_months > 0, Error::ReturnMonthsMustBeGreaterThanZero,
            min_per_investment > 0, Error::MinPerInvestmentMustBeGreaterThanZero
        );

        let ret_type = InvestmentReturnType::from_number(return_type).ok_or(Error::UnsupportedReturnType)?;

        let contract_data = ContractData {
            interest_rate: i_rate,
            claim_block_days,
            token: token_addr,
            project_address,
            admin: admin_addr,
            state: State::Actve,
            return_type: ret_type,
            return_months,
            min_per_investment,
            goal,
        };

        update_contract_data(&env, &contract_data);
        Ok(())
    }

    /// Processes a scheduled payment to an investor (admin only).
    ///
    /// Transfers the regular payment amount from the contract's reserve balance to the investor.
    /// Updates investment status, payment tracking, and claim schedules. Validates timing constraints
    /// to ensure payments are made according to the investment schedule.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address receiving the payment.
    /// * `ts` - The claimable timestamp identifying the specific investment.
    ///
    /// # Returns
    ///
    /// * The updated `Investment` object with incremented payment counters.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for this address and timestamp.
    /// * `AddressInvestmentIsNotClaimableYet` if the claimable date hasn't been reached.
    /// * `AddressInvestmentIsFinished` if all payments have been completed.
    /// * `AddressInvestmentNextTransferNotClaimableYet` if less than a month has passed since last payment.
    /// * `ContractInsufficientBalance` if reserve balance is insufficient.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if token transfer fails.
    pub fn process_investor_payment(env: Env, addr: Address, ts: u64) -> Result<Investment, Error> {
        let contract_data = require_admin(&env);

        let mut investment = get_investment(&env, &addr, ts).ok_or(Error::AddressHasNotInvested)?;

        require!(
            env.ledger().timestamp() >= investment.claimable_ts, Error::AddressInvestmentIsNotClaimableYet,
            investment.status != InvestmentStatus::Finished, Error::AddressInvestmentIsFinished,
            investment.last_transfer_ts == 0 || (env.ledger().timestamp() - investment.last_transfer_ts) >= SECONDS_IN_MONTH, Error::AddressInvestmentNextTransferNotClaimableYet
        );

        let mut contract_balances: ContractBalances = get_balances_or_new(&env);
        let tk = get_token(&env, &contract_data);
        let amount_to_transfer: i128 = process_investment_payment(&env, &mut investment, &contract_data);

        require!(amount_to_transfer <= contract_balances.reserve, Error::ContractInsufficientBalance);
        tk.try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?
        ;

        update_investment(&env, &addr, &investment);
        decrement_project_balance_from_payment_to_investor(&mut contract_balances, &amount_to_transfer);
        update_contract_balances(&env, &contract_balances);

        env.events().publish((TOPIC_CONTRACT_BALANCE_UPDATED,), contract_balances);
        Ok(investment)
    }

    /// Allows an investor to make a new investment.
    ///
    /// Validates the investment amount, contract state, and funding goal constraints.
    /// Transfers tokens from the investor to the contract, splits them into project and reserve balances,
    /// creates the investment record with calculated returns, and updates the contract state.
    /// If the funding goal is reached, changes contract state to 'FundsReached'.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address (requires authentication).
    /// * `amount` - The investment amount in tokens.
    ///
    /// # Returns
    ///
    /// * The created `Investment` object with all calculated fields.
    ///
    /// # Errors
    ///
    /// * `AmountLessThanMinimum` if amount is below the minimum per investment.
    /// * `ContractMustBeActiveToInvest` if contract is paused or funding is reached.
    /// * `AddressInsufficientBalance` if investor doesn't have enough tokens.
    /// * `WouldExceedGoal` if this investment would exceed the funding goal.
    pub fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error> {
        addr.require_auth();
        let mut contract_data: ContractData = get_contract_data(&env);
        let tk = get_token(&env, &contract_data);

        require!(
            amount >= contract_data.min_per_investment, Error::AmountLessThanMinimum,
            contract_data.state == State::Actve, Error::ContractMustBeActiveToInvest,
            tk.balance(&addr) >= amount,Error::AddressInsufficientBalance
        );


        let token_decimals = tk.decimals();
        let amounts: Amount = Amount::from_investment(&amount, &contract_data.interest_rate, token_decimals);
        
        // Validate goal before transfer
        let mut contract_balances = get_balances_or_new(&env);
        let invested_amount = amounts.amount_to_invest + amounts.amount_to_reserve_fund;
        require!(
            contract_balances.received_so_far + invested_amount <= contract_data.goal,
            Error::WouldExceedGoal
        );

        tk.try_transfer(&addr, &env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        recalculate_contract_balances_from_investment(&mut contract_balances, &amounts);
        update_contract_balances(&env, &contract_balances);

        let addr_investment: Investment = build_investment(&env, &contract_data, &amount, token_decimals);
        update_investment(&env, &addr, &addr_investment);

        if contract_balances.received_so_far >= contract_data.goal {
            contract_data.state = State::FundsReached;
            update_contract_data(&env, &contract_data);
            env.events().publish((TOPIC_CONTRACT_STATUS_UPDATED,), contract_data.state);
        }

        env.events().publish((TOPIC_CONTRACT_BALANCE_UPDATED,), contract_balances);

        Ok(addr_investment)
    }

    /// Retrieves the current contract balances (admin only).
    ///
    /// Returns the breakdown of contract funds across different balance categories:
    /// project balance, reserve balance, commission, and total received.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * `ContractBalances` containing all balance information.
    pub fn get_contract_balance(env: Env) -> Result<ContractBalances, Error> {
        require_admin(&env);

        let contract_balances: ContractBalances = get_balances_or_new(&env);

        Ok(contract_balances)
    }

    /// Pauses new investments (admin only).
    ///
    /// Changes the contract state from 'Active' to 'Paused', preventing new investments
    /// while existing investments continue to function normally.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `ContractMustBeActiveToBePaused` if the contract is not in 'Active' state.
    pub fn stop_investments(env: Env) -> Result<bool, Error> {
        let mut contract_data = require_admin(&env);
        require!(contract_data.state == State::Actve, Error::ContractMustBeActiveToBePaused);
        contract_data.state = State::Paused;
        update_contract_data(&env, &contract_data);

        Ok(true)
    }

    /// Resumes accepting new investments.
    ///
    /// Allows the admin to change the contract state back to 'Active', which allows new investments again.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * Returns `true` on success, or an error if something goes wrong.
    pub fn restart_investments(env: Env) -> Result<bool, Error> {
        let mut contract_data = require_admin(&env);
        require!(contract_data.state == State::Paused, Error::ContractMustBePausedToRestartAgain);
        contract_data.state = State::Actve;
        update_contract_data(&env, &contract_data);

        Ok(true)
    }

    /// Withdraws funds from the project balance to the project address (admin only).
    ///
    /// Transfers the specified amount from the contract's project balance to the configured
    /// project address. Validates sufficient balance and updates internal accounting.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to withdraw from project balance.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `ContractInsufficientBalance` if project balance is less than the requested amount.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the transfer fails.
    pub fn single_withdrawn(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data = require_admin(&env);

        let mut contract_balances: ContractBalances = get_balances_or_new(&env);
        require!(contract_balances.project >= amount, Error::ContractInsufficientBalance);

        let tk = get_token(&env, &contract_data);

        // Verify the transfer can be completed
        tk.try_transfer(
            &env.current_contract_address(),
            &contract_data.project_address,
            &amount,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;
        
        decrement_project_balance_from_company_withdrawal(&mut contract_balances, &amount);
        update_contract_balances(&env, &contract_balances);
        env.events().publish((TOPIC_CONTRACT_BALANCE_UPDATED,), contract_balances);

        Ok(true)
    }

    /// Calculates additional funds needed in reserve balance (admin only).
    ///
    /// Analyzes upcoming payment claims (within the next week) and compares them against
    /// the current reserve balance to determine if additional funds are needed.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * The additional amount needed in reserve, or 0 if reserve is sufficient.
    pub fn check_reserve_balance(env: Env) -> Result<i128, Error> {
        require_admin(&env);

        let claims_map: Map<Address, Claim> = get_claims_map_or_new(&env);
        let project_balances: ContractBalances = get_balances_or_new(&env);
        let mut min_funds: i128 = 0;

        for (_addr, next_claim) in claims_map.iter() {
            if next_claim.is_claim_next(&env) {
                min_funds += next_claim.amount_to_pay;
            }
        }

        if min_funds > 0 {
            if project_balances.reserve < min_funds {
                let diff_to_contribute: i128 = min_funds - project_balances.reserve;
                return Ok(diff_to_contribute);
            }
        }

        Ok(0_i128)
        
    }

    /// Adds funds from admin to the contract's reserve balance (admin only).
    ///
    /// Transfers tokens from the admin address to the contract and adds them to the reserve balance.
    /// This is used to replenish the reserve fund for upcoming investor payments.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to transfer to reserve.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `AddressInsufficientBalance` if admin doesn't have enough tokens.
    pub fn add_company_transfer(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data = require_admin(&env);

        let tk = get_token(&env, &contract_data);
        require!(tk.balance(&contract_data.admin) >= amount, Error::AddressInsufficientBalance);
        tk.try_transfer(&contract_data.admin, &env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let mut contract_balances = get_balances_or_new(&env);
        increment_reserve_balance_from_company_contribution(&mut contract_balances, &amount);
        update_contract_balances(&env, &contract_balances);
        env.events().publish((TOPIC_CONTRACT_BALANCE_UPDATED,), contract_balances);

        Ok(true)
    }

    /// Moves funds from project balance to reserve balance (admin only).
    ///
    /// Transfers the specified amount internally from the project balance to the reserve balance.
    /// This is used to ensure sufficient reserve funds for upcoming investor payments.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to move from project to reserve.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `ProjectBalanceInsufficientAmount` if project balance is less than the requested amount.
    pub fn move_funds_to_the_reserve(env: Env, amount: i128) -> Result<bool, Error> {
        require_admin(&env);

        let mut contract_balances = get_balances_or_new(&env);
        require!(
            contract_balances.project > amount,
            Error::ProjectBalanceInsufficientAmount
        );

        move_from_project_balance_to_reserve_balance(&mut contract_balances, &amount);
        update_contract_balances(&env, &contract_balances);
        env.events().publish((TOPIC_CONTRACT_BALANCE_UPDATED,), contract_balances);

        Ok(true)
    }
}
