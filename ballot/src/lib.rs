#![no_std]

use soroban_sdk::{contract, contractimpl, contracterror, Env, Symbol, Map, Address, Vec};

mod storage;
mod validation;
use storage::VCounter;

struct Voter<'a> {
    id: &'a Symbol
}

impl<'a> Voter<'a> {
    
    fn has_voted(&self, env: &Env) -> bool {
        let vts: Vec<Symbol> = storage::get_votes(env);
        vts.contains(self.id)
    }

    fn is_delegated(&self, env: &Env) -> bool {
        let dvts: Vec<Symbol> = storage::get_delegated_votes(env);
        dvts.contains(self.id)
    }

    fn has_delegated_votes(&self, env: &Env) -> bool {
        let dvotes = storage::get_voter_delegated_votes(env, self.id);
        if dvotes.len() > 0 {
            return true;
        }
    
        false
    }
}

fn check_dates(env: &Env) -> bool {
    let cfg = storage::get_config(env);
    let mut valid = true;
    if cfg.from > 0 && cfg.to > 0 {
        valid = validation::is_valid_date(&env, &cfg.from, &cfg.to)
    }

    valid
}


#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    VoterHasHisVoteDelegated = 1,
    VoterHasAlreadyVoted = 2,
    VoterHasDelegatedVotes = 3,
    VoterOriginHasAlreadyVotedAndCannotDelegate = 4,
    VoterTargetHasAlreadyVotedAndCannotDelegate = 5,
    BallotOutOfDate = 6

}

#[contract]
pub struct Ballot;

#[contractimpl]
impl Ballot {

    pub fn configure(env: Env, admin: Address, ts_start: u64, ts_end: u64) -> Result<bool, Error> {
        admin.require_auth();
        storage::store_config(&env, ts_start, ts_end);
        Ok(true)
    }

    pub fn vote(env: Env, admin: Address, voter: Symbol, candidate: Symbol) -> Result<bool, Error> {
        admin.require_auth();
        
        if !check_dates(&env) {
            return Err(Error::BallotOutOfDate);
        }

        let v: Voter = Voter { id: &voter };

        if v.is_delegated(&env) {
            return Err(Error::VoterHasHisVoteDelegated)
        }
        if v.has_voted(&env) {
            return Err(Error::VoterHasAlreadyVoted)
        }
        
        storage::store_party(&env, &candidate);

        let mut votes: Vec<Symbol> = storage::get_votes(&env);
        let candidate_key = VCounter::Counter(candidate);
        let d_votes: Vec<Symbol> = storage::get_voter_delegated_votes(&env, v.id);
        let count = 1 + d_votes.len() + storage::get_candidate_votes_count(&env, &candidate_key);
        votes.push_back(voter);
 
        storage::update_candidate_count(&env, candidate_key, count);
        storage::update_votes(&env, votes);

        Ok(true)
    }

    pub fn delegate(env: Env,  admin: Address, o_voter: Symbol, d_voter: Symbol) -> Result<bool, Error> {
        admin.require_auth();

        if !check_dates(&env) {
            return Err(Error::BallotOutOfDate);
        }

        let ov: Voter = Voter { id: &o_voter };
        let dv: Voter = Voter { id: &d_voter };
    
        
        if ov.has_voted(&env) {
            return Err(Error::VoterOriginHasAlreadyVotedAndCannotDelegate)
        }

        if dv.has_voted(&env) {
            return Err(Error::VoterTargetHasAlreadyVotedAndCannotDelegate)
        }

        if ov.is_delegated(&env) {
            return Err(Error::VoterHasHisVoteDelegated)
        }

        if dv.is_delegated(&env) {
            return Err(Error::VoterHasHisVoteDelegated)
        }

        if ov.has_delegated_votes(&env) {
            return Err(Error::VoterHasDelegatedVotes)
        }


        let mut d_votes = storage::get_delegated_votes(&env);
        let mut d_vot_delegs: Vec<Symbol> = storage::get_voter_delegated_votes(&env, &d_voter);
        d_votes.push_back(o_voter.clone());
        d_vot_delegs.push_back(o_voter.clone());

        storage::update_delegated_votes(&env, d_votes);
        storage::update_voter_delegated_votes(&env, d_voter, d_vot_delegs);

        Ok(true)

    }

    pub fn count(env: Env,  admin: Address) -> Map<Symbol, u32> {
        
        admin.require_auth();
        let pts = storage::get_candidates(&env);
        let mut count_map: Map<Symbol, u32>= Map::new(&env);
        for party in pts.iter() {
            let candidate_key = VCounter::Counter(party.clone());
            let candidate_count: u32 = storage::get_candidate_votes_count(&env, &candidate_key);
            count_map.set(party, candidate_count);
        }

        count_map
    }
}

mod test;
