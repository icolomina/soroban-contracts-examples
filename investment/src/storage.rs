use crate::{balance::ContractBalances, claim::Claim, data::{ContractData, DataKey}, investment::Investment, multisig::MultisigRequest};
use soroban_sdk::{Address, Env, Map, String};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;


pub fn get_contract_data(e: &Env) -> ContractData {
    if let Some(contract_data) = e.storage().instance().get(&DataKey::ContractData) {
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        return contract_data;
    }
    
    panic!("Contract not initialized");
}

pub fn update_contract_data(e: &Env, contract_data: &ContractData) {
    e.storage().instance().set(&DataKey::ContractData, contract_data);
    e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn has_investment(e: &Env, addr: Address) -> bool {
    let key = DataKey::Investment(addr);
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        return true;
    }

    false
}

pub fn get_investment(e: &Env, addr: Address) -> Option<Investment> {
    let key = DataKey::Investment(addr);
    if let Some(investment_data) = e.storage().persistent().get(&key).unwrap() {
        e.storage().persistent().extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        return Some(investment_data);
    }

    None
}

pub fn set_investment(e: &Env, addr: Address, investment: &Investment) {
    let key = DataKey::Investment(addr);
    e.storage().persistent().set(&key, investment);
    e.storage().persistent().extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn update_claims_map(e: &Env, claims_map: Map<Address, Claim>) {
    let claims_map_key = DataKey::ClaimsMap;
    e.storage().instance().set(&claims_map_key, &claims_map);
    e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn get_claims_map_or_new(e: &Env) -> Map<Address, Claim> {
    let claims_map = e.storage().instance().get(&DataKey::ClaimsMap);
    match claims_map {
        Some(x) => x,
        None => Map::<Address, Claim>::new(&e)
    }
}

pub fn update_contract_balances(e: &Env, contract_balances: &ContractBalances) {
    e.storage().instance().set(&DataKey::ContractBalances, contract_balances);
    e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn get_balances(e: &Env) -> Option<ContractBalances> {
    if let Some(contract_balances) = e.storage().instance().get(&DataKey::ContractBalances) {
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        return Some(contract_balances);
    }

    None
}

pub fn get_balances_or_new(e: &Env) -> ContractBalances {
    let contract_balances = get_balances(&e);
    match contract_balances {
        Some(x) => x,
        None => ContractBalances::new()
    }
}

pub fn get_multisig_or_new(e: &Env, contract_data: &ContractData, multisig_claim: String, successful_signatures: u32, amount: i128, valid_ts: u64) -> MultisigRequest {
    let multisig_request = e.storage().temporary().get(&DataKey::MultisigRequest);
    match multisig_request {
        Some(x) => x,
        None => MultisigRequest::new(
            &e, 
            &contract_data, 
            multisig_claim, 
            successful_signatures, 
            amount,
            valid_ts
        )
    }
}


