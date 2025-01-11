use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, log, token, Address, Env, Map, String};

use crate::balance::{recalculate_contract_balances_from_amount, Amount, CalculateAmounts, ContractBalances};
use crate::claim::{calculate_next_claim, Claim};
use crate::data::{
    ContractData, DataKey, Error, State, FromNumber
};
use crate::investment::{build_investment, process_investment_claim, Investment, InvestmentReturnType, InvestmentStatus};
use crate::multisig::{MultisigRequest, MultisigStatus};
use crate::storage::{
    get_balances_or_new, 
    get_claims_map_or_new, 
    get_contract_data, 
    get_investment, 
    get_multisig_or_new, 
    has_investment, 
    set_investment, 
    update_claims_map, 
    update_contract_balances, 
    update_contract_data
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

fn decrement_project_balance(e: &Env, amount: i128) {
    let mut contract_balances = get_balances_or_new(e);
    contract_balances.project -= amount;
    update_contract_balances(e, &contract_balances);
}

fn write_balances(e: &Env, amounts: &Amount) {
    let mut contract_balances = get_balances_or_new(e);
    recalculate_contract_balances_from_amount(&mut contract_balances, amounts);

    update_contract_balances(e, &contract_balances);
}


#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {

    pub fn __constructor(env: Env, admin_addr: Address, project_address: Address, token_addr: Address, i_rate: u32, claim_block_days: u64, goal: i128, return_type: u32, return_months: u32, min_per_investment: i128) {

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
                goal
            };

            update_contract_data(&env, &contract_data);
        } else {
            //return Err(Error::UnsupportedReturnType);
            panic!("unsupported return type");
        }

    }

    pub fn claim(env: Env, addr: Address) -> Result<Investment, Error>
    {
        require!(has_investment(&env, addr.clone()), Error::ContractAlreadyInitialized);

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let mut investment: Investment;
        match get_investment(&env, addr.clone()) {
            Some(inv) => investment = inv,
            None => return Err(Error::AddressHasNotInvested)
        } 
        require!(env.ledger().timestamp() >= investment.claimable_ts, Error::AddressInvestmentIsNotClaimableYet);
        require!(investment.status != InvestmentStatus::Finished, Error::AddressInvestmentIsFinished);

        let seconds_in_a_month = 30 * 24 * 60 * 60;
        if investment.last_transfer_ts == 0 || (env.ledger().timestamp() - investment.last_transfer_ts) > seconds_in_a_month {

            let tk = get_token(&env);
            let amount_transferred: i128 = process_investment_claim(&env, &mut investment, &contract_data, &tk, &addr);
            update_investment(&env, addr.clone(), &investment);
            decrement_project_balance(&env, amount_transferred);
            log!(&env, "En el contrato queda: {}", tk.balance(&env.current_contract_address()));
            return Ok(investment);
        } else {
            return Err(Error::AddressInvestmentNextTransferNotClaimableYet);
        }

    }

    pub fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error> {

        require!(amount > 0, Error::AmountLessOrEqualThan0);

        let contract_data: ContractData = get_contract_data(&env);
        require!(contract_data.state != State::FinancingReached, Error::ContractFinancingReached);
        require!(!has_investment(&env, addr.clone()), Error::AddressAlreadyInvested);

        addr.require_auth();
        let tk = get_token(&env);

        require!(contract_data.goal == 0 || tk.balance(&env.current_contract_address()) < contract_data.goal, Error::ContractHasReachedInvestmentGoal);
        require!(tk.balance(&addr) >= amount, Error::AddressInsufficientBalance);

        let amounts: Amount = Amount::from_investment(&amount);
        tk.transfer(&addr, &env.current_contract_address(), &amount);

        write_balances(&env, &amounts);
        
        let addr_investment: Investment = build_investment(&env, &contract_data, &amount);
        update_investment(&env, addr.clone(), &addr_investment);

        Ok(addr_investment)
    }

    pub fn get_contract_balance(env: Env) -> Result<ContractBalances, Error> {

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let contract_balances: ContractBalances = get_balances_or_new(&env);

        Ok(contract_balances)
    }

    pub fn stop_investments(env: Env) -> Result<bool, Error> {

        let mut contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();
        contract_data.state = State::FinancingReached;
        update_contract_data(&env, &contract_data);

        Ok(true)
    }

    pub fn project_withdrawn(env: Env, addr: Address, amount: i128) -> Result<MultisigStatus, Error> {

        let valid_ts = env.ledger().timestamp() + 86400;

        let tk = get_token(&env);
        let contract_balances: ContractBalances = get_balances_or_new(&env);

        require!(contract_balances.project > amount, Error::ContractInsufficientBalance);

        let contract_data: ContractData = get_contract_data(&env);
        let multisig_claim : String = String::from_str(&env, "project_withdrawn");
        let mut multisig: MultisigRequest = get_multisig_or_new(&env, &contract_data, multisig_claim, 2_u32, amount, valid_ts);

        require!(multisig.is_valid_signature(addr.clone()), Error::AddressAlreadyDeposited);
        if multisig.is_completed() {
            env.storage().temporary().remove(&DataKey::MultisigRequest);
            tk.transfer(&env.current_contract_address(), &contract_data.project_address, &amount); 
            return Ok(MultisigStatus::Completed)
        }

        addr.require_auth();

        multisig.add_sig(addr);
        env.storage().temporary().set(&DataKey::MultisigRequest, &multisig);
        Ok(MultisigStatus::WaitingForSignatures)

    }

    pub fn check_project_address_balance(env: Env) -> Result<i128, Error> {

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let claims_map: Map<Address, Claim> = get_claims_map_or_new(&env);
        let project_balances: ContractBalances = get_balances_or_new(&env);
        let mut min_funds: i128 = 0;

        let tk = get_token(&env);
        let project_address_balance = tk.balance(&env.current_contract_address()) - project_balances.comission - project_balances.reserve_fund;

        for (_addr, next_claim) in claims_map.iter() {
            if next_claim.is_claim_next(&env) {
                min_funds += next_claim.amount_to_pay;
            }
        }

        if min_funds > 0 {
            if project_address_balance < min_funds {
                return Err(Error::ProjectAddressInsufficientBalance);
            }
        }

        Ok(project_address_balance)
    }
}