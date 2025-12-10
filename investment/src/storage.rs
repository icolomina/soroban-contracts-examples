use crate::{balance::ContractBalances, claim::Claim, data::{ContractData, DataKey}, investment::Investment};
use soroban_sdk::{Address, Env, Map};

pub(self) const DAY_IN_LEDGERS: u32 = 17280;

// Instance storage: accessed frequently, moderate TTL
pub(self) const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;      // ~30 days
pub(self) const INSTANCE_LIFETIME_THRESHOLD: u32 = 15 * DAY_IN_LEDGERS; // ~15 days

// Persistent storage: critical user data, long TTL for safety
pub(self) const PERSISTENT_BUMP_AMOUNT: u32 = 180 * DAY_IN_LEDGERS;    // ~6 months
pub(self) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 90 * DAY_IN_LEDGERS; // ~3 months


pub fn get_contract_data(e: &Env) -> ContractData {
    let contract_data = e.storage()
        .instance()
        .get(&DataKey::ContractData)
        .unwrap_or_else(|| panic!("Contract data has expired"));
    
    bump_instance_ttl(e);
    contract_data
}

pub fn update_contract_data(e: &Env, contract_data: &ContractData) {
    e.storage().instance().set(&DataKey::ContractData, contract_data);
}

pub fn get_investment(e: &Env, addr: &Address, ts: u64) -> Option<Investment> {
    let key = DataKey::Investment(addr.clone());
    let addr_investments: Option<Map<u64, Investment>> = e.storage().persistent().get(&key);
    
    if let Some(investments) = addr_investments {
        bump_persistent_ttl(e, &key);
        investments.get(ts)
    } else {
        None
    }
}

pub fn set_investment(e: &Env, addr: &Address, investment: &Investment) {
    let key = DataKey::Investment(addr.clone());

    let mut addr_investments = e.storage().persistent().get(&key).unwrap_or(Map::<u64, Investment>::new(&e));
    addr_investments.set(investment.claimable_ts, *investment);

    e.storage().persistent().set(&key, &addr_investments);
}

pub fn update_claims_map(e: &Env, claims_map: Map<Address, Claim>) {
    e.storage().instance().set(&DataKey::ClaimsMap, &claims_map);
}

pub fn get_claims_map_or_new(e: &Env) -> Map<Address, Claim> {
    let key = DataKey::ClaimsMap;
    let claims_map = e.storage().instance()
        .get(&key) 
        .unwrap_or(Map::<Address, Claim>::new(&e))
    ;

    bump_instance_ttl(e);
    claims_map        
}

pub fn update_contract_balances(e: &Env, contract_balances: &ContractBalances) {
    e.storage().instance().set(&DataKey::ContractBalances, contract_balances);
}

pub fn get_balances_or_new(e: &Env) -> ContractBalances {

    let key = DataKey::ContractBalances;
    let contract_balances = e.storage().instance()
        .get(&key) 
        .unwrap_or(ContractBalances::new())
    ;

    bump_instance_ttl(e);
    contract_balances
}

fn bump_instance_ttl(e: &Env) {
    e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

fn bump_persistent_ttl(e: &Env, key: &DataKey) {
    e.storage().persistent().extend_ttl(key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}



