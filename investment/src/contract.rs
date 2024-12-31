use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, log, token, Address, Env, Map, String};
use crate::data::{
    Amount, CalculateAmounts, Claim, ContractBalances, ContractData, DataKey, Error, FromNumber, Investment, InvestmentReturnType, InvestmentStatus, MultisigRequest, MultisigStatus, State, INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD
};
use crate::investment::{build_investment, process_investment_claim};

macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

fn check_contract_initialized(e: &Env) -> bool {
    let contract_data_key = DataKey::ContractData;
    if e.storage().instance().has(&contract_data_key) {
        return true;
    }

    false
}

fn get_contract_data(e: &Env) -> ContractData {
    let contract_data_key = DataKey::ContractData;
    let contract_data = e.storage().instance().get(&contract_data_key).unwrap();
    contract_data
}

fn update_contract_data(e: &Env, contract_data: &ContractData) {
    let contract_data_key = DataKey::ContractData;
    e.storage().instance().set(&contract_data_key, contract_data);
    e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

fn get_token(env: &Env) -> TokenClient {
    let contract_data = get_contract_data(&env);
    let tk = token::Client::new(&env, &contract_data.token);
    tk
}

fn has_investment(e: &Env, addr: Address) -> bool {
    let investment_key = DataKey::Investment(addr);
    if e.storage().persistent().has(&investment_key) {
        return true;
    }

    return false;
}

fn get_investment(e: &Env, addr: Address) -> Investment {
    let investment_key = DataKey::Investment(addr);
    let investment_data = e.storage().persistent().get(&investment_key).unwrap();
    investment_data
}

fn update_investment(e: &Env, addr: Address, investment: &Investment) {
    let investment_key = DataKey::Investment(addr.clone());
    let claims_map_key = DataKey::ClaimsMap;
    e.storage().persistent().set(&investment_key, investment);
    let seconds_in_a_month = 30 * 24 * 60 * 60;
    let mut claims_map: Map<Address, Claim> = e.storage().instance().get(&claims_map_key).unwrap_or(Map::<Address, Claim>::new(&e));
    let next_claim = Claim {
        next_transfer_ts: match investment.last_transfer_ts {
            lts if lts > 0  => lts + seconds_in_a_month,
            _ => e.ledger().timestamp() + seconds_in_a_month
        },
        amount_to_pay: investment.regular_payment
    };

    claims_map.set(addr.clone(), next_claim);
    e.storage().instance().set(&claims_map_key, &claims_map);

}

fn decrement_project_balance(env: &Env, amount: i128) {
    let mut contract_balances = read_balances(&env);
    contract_balances.project -= amount;
    env.storage().instance().set(&DataKey::ContractBalances, &contract_balances);
}

fn write_balances(env: &Env, amounts: &Amount) {
    let mut contract_balances = read_balances(env);
    contract_balances.comission += amounts.amount_to_commission;
    contract_balances.reserve_fund += amounts.amount_to_reserve_fund;
    contract_balances.project += amounts.amount_to_invest;

    env.storage().instance().set(&DataKey::ContractBalances, &contract_balances);
}

fn read_balances(env: &Env) -> ContractBalances {
    let contract_balances = env.storage().instance().get(&DataKey::ContractBalances).unwrap_or(ContractBalances::new());
    contract_balances
}


#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {

    pub fn init(env: Env, admin_addr: Address, project_address: Address, token_addr: Address, i_rate: u32, claim_block_days: u64, goal: i128, return_type: u32, return_months: u32, min_per_investment: i128) -> Result<bool, Error>{

        require!(!check_contract_initialized(&env), Error::ContractAlreadyInitialized);
        require!(i_rate > 0, Error::RateMustBeGreaterThan0);

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
            return Err(Error::UnsupportedReturnType);
        }

        Ok(true)
    }

    pub fn claim(env: Env, addr: Address) -> Result<Investment, Error>
    {
        require!(check_contract_initialized(&env), Error::ContractAlreadyInitialized);
        require!(has_investment(&env, addr.clone()), Error::ContractAlreadyInitialized);

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let mut investment: Investment = get_investment(&env, addr.clone());
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

        require!(check_contract_initialized(&env), Error::ContractAlreadyInitialized);
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

        require!(check_contract_initialized(&env), Error::ContractNotInitialized);

        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let contract_balances: ContractBalances = read_balances(&env);

        Ok(contract_balances)
    }

    pub fn stop_investments(env: Env) -> Result<bool, Error> {

        require!(check_contract_initialized(&env), Error::ContractNotInitialized);

        let mut contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();
        contract_data.state = State::FinancingReached;
        update_contract_data(&env, &contract_data);

        Ok(true)
    }

    pub fn project_withdrawn(env: Env, addr: Address, amount: i128) -> Result<MultisigStatus, Error> {

        let valid_ts = env.ledger().timestamp() + 86400;

        let tk = get_token(&env);
        let contract_balances: ContractBalances = read_balances(&env);

        require!(contract_balances.project > amount, Error::ContractInsufficientBalance);

        let contract_data: ContractData = get_contract_data(&env);
        let mut multisig: MultisigRequest = env.storage().temporary().get(&DataKey::MultisigRequest).unwrap_or(
            MultisigRequest::new(
                &env, 
                &contract_data, 
                String::from_str(&env, "project_withdrawn"), 
                2_u32, 
                amount,
                valid_ts
            )
        );

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

        require!(check_contract_initialized(&env), Error::ContractNotInitialized);
        let contract_data: ContractData = get_contract_data(&env);
        contract_data.admin.require_auth();

        let claims_map: Map<Address, Claim> = env.storage().instance().get(&DataKey::ClaimsMap).unwrap_or(Map::<Address, Claim>::new(&env));
        let project_balances: ContractBalances = read_balances(&env);
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