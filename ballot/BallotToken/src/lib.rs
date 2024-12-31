#![no_std]

use soroban_sdk::{contract, contracttype, contractimpl, contracterror, symbol_short, Address, Env, Symbol, Vec, vec, String};

const TOPIC_BALLOT: Symbol = symbol_short!("BALLOT");
const TOPIC_DELEGATION_REQUESTED: Symbol = symbol_short!("D_REQ");

pub const DAY_IN_LEDGERS: u32 = 17280;
pub const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    TokenAlreadyInitialized = 1,
    AddressAlreadyHoldsToken = 2,
    AddressDoesNotHoldToken = 3,
    AddressAlreadyHasAllowance = 4,
    ExpirationLedgerLessThanCurrentLedger = 5
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Owner(Address),
    Delegated(Address),
    Delegations(Address),
    CurrentBallot,
    Admin,
    RequestDelegation(RequestedDelegation),
    ExpirationLedger
}

#[derive(Clone)]
#[contracttype]
pub struct RequestedDelegation {
    from: Address,
    to: Address
}

pub struct OwnerBallotInfo<'a> {
    owner: &'a Address
}

impl<'a> OwnerBallotInfo<'a> {
    fn count_delegations(&self, e: &Env) -> u32 {
        let delegations_key = DataKey::Delegations(self.owner.clone());
        if let Some(delegations) = e.storage().temporary().get::<DataKey, Vec<Address>>(&delegations_key) {
            return delegations.len();
        }
    
        0
    }

    fn add_delegation(&self, e: &Env, addr: Address) -> bool {

        let from_delegated_key = DataKey::Delegated(addr.clone());
        e.storage().temporary().set(&from_delegated_key, &true);
    
        let to_delegations_key = DataKey::Delegations(self.owner.clone());
        if let Some(mut to_delegations) = e.storage().temporary().get::<DataKey, Vec<Address>>(&to_delegations_key) {
            to_delegations.push_back(addr);
            e.storage().temporary().set(&to_delegations_key, &to_delegations);
        } else {
            let to_delegations = vec![&e, addr];
            e.storage().temporary().set(&to_delegations_key, &to_delegations);
        }
    
        true
    }

    fn is_delegated(&self, e: &Env) -> bool {
        let blocking_key = DataKey::Delegated(self.owner.clone());
        if let Some(_b) = e.storage().temporary().get::<_, Address>(&blocking_key) {
            return true;
        }
    
        false
    }

    fn request_delegation(&self, e: &Env, addr: Address) {
        let requested_delegation = RequestedDelegation {
            from: addr,
            to: self.owner.clone()
        };

        let request_delegation_key = DataKey::RequestDelegation(requested_delegation.clone());
        e.storage().temporary().set(&request_delegation_key, &false);
        e.events().publish((TOPIC_BALLOT, TOPIC_DELEGATION_REQUESTED), requested_delegation.clone());
    }
}

fn has_admin(e: &Env) -> bool {
    let admin_key = DataKey::Admin;
    let has_admin = e.storage().instance().has(&admin_key);
    has_admin
}


fn is_ballot_running(e: &Env) -> bool {
    let current_ballot_key = DataKey::CurrentBallot;
    let exists = e.storage().temporary().has(&current_ballot_key);

    exists
}

fn is_owner(e: &Env, addr: Address) -> bool {
    let owner_key = DataKey::Owner(addr);
    if let Some(_owner) = e.storage().instance().get::<DataKey, u32>(&owner_key) {
        return true;
    }
    
    false
}


#[contract]
pub struct BallotToken;

#[contractimpl]
impl BallotToken {

    pub fn initialize(e: Env, admin: Address) -> Result<bool, Error> {

        if has_admin(&e) {
            return Err(Error::TokenAlreadyInitialized);
        }

        e.storage().instance().set(&DataKey::Admin, &admin);
        Ok(true)
        
    }

    pub fn load_ballot(e: Env, id: String, expiration_ledger: u32) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if is_ballot_running(&e) {
            // Error -> BallotAlreadyRunning
        }

        let current_ballot_key = DataKey::CurrentBallot;
        let expiration_ledger_key = DataKey::ExpirationLedger;

        e.storage().temporary().set(&current_ballot_key, &id);
        e.storage().temporary().extend_ttl(&current_ballot_key, expiration_ledger, expiration_ledger);
        e.storage().temporary().set(&expiration_ledger_key, &expiration_ledger);
    }

    pub fn get_current_ballot(e: Env) -> String {
        let current_ballot_key = DataKey::CurrentBallot;
        let current_ballot = e.storage().temporary().get::<DataKey, String>(&current_ballot_key).unwrap_or(String::from_str(&e, ""));

        current_ballot
    }

    pub fn mint(e: Env, addr: Address) -> Result<bool, Error> {
        
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if is_owner(&e, addr.clone()) {
            return Err(Error::AddressAlreadyHoldsToken);
        }

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        e.storage().persistent().set(&DataKey::Owner(addr.clone()), &1_u32);
        Ok(true)
    }

    pub fn can_vote(e: Env, addr: Address) -> bool {
        let owner_key = DataKey::Owner(addr);
        if let Some(_b) = e.storage().persistent().get::<DataKey, u32>(&owner_key) {
            let current_ballot_key = DataKey::CurrentBallot;
            if let Some(_id) = e.storage().temporary().get::<DataKey, String>(&current_ballot_key) {
                e.storage().persistent().extend_ttl(&owner_key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
                return true
            }
        }

        false
    }

    pub fn get_addr_delegations(e: Env, addr: Address) -> u32 {
        let key = DataKey::Owner(addr.clone());
        if let Some(_b) = e.storage().persistent().get::<DataKey, u32>(&key) {
            let owner_ballot_info = OwnerBallotInfo { owner: &addr };
            return owner_ballot_info.count_delegations(&e);
        }

        0
    }

    pub fn request_delegation(e: Env, from: Address, to: Address) -> Result<bool, Error> {

        if !is_owner(&e, from.clone()) {
            return Err(Error::AddressDoesNotHoldToken);
        }

        if !is_owner(&e, to.clone()) {
            return Err(Error::AddressDoesNotHoldToken);
        }

        from.require_auth();
        let from_ballot_info = OwnerBallotInfo { owner: &from };
        let to_ballot_info = OwnerBallotInfo { owner: &to };

        if from_ballot_info.count_delegations(&e) > 0{
            // From has delegated votes so it cannot delegate its vote
        }

        if from_ballot_info.is_delegated(&e) {
            // From has already delegated its vote
        }

        if to_ballot_info.is_delegated(&e) {
            // To has already delegated its vote
        }

        from_ballot_info.request_delegation(&e, from.clone());
        Ok(true)
    }

    pub fn approve_delegation(e: Env, from: Address, to: Address) -> Result<bool, Error> {
        if !is_owner(&e, from.clone()) {
            return Err(Error::AddressDoesNotHoldToken);
        }

        if !is_owner(&e, to.clone()) {
            return Err(Error::AddressDoesNotHoldToken);
        }

        to.require_auth();
        let to_ballot_info = OwnerBallotInfo { owner: &to };
        to_ballot_info.add_delegation(&e, from.clone());
        Ok(true)
    }

    pub fn burn(e: Env, addr: Address) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let owner_key = DataKey::Owner(addr);
        e.storage().persistent().remove(&owner_key);
    }

}

mod test;
