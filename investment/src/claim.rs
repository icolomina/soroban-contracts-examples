use soroban_sdk::{contracttype, Env};
use crate::constants::{SECONDS_IN_MONTH, SECONDS_IN_WEEK};
use crate::investment::Investment;

#[contracttype]
#[derive(Copy, Clone)]
pub struct Claim {
    pub next_transfer_ts: u64,
    pub amount_to_pay: i128
}

impl Claim {
    pub fn is_claim_next(&self, env: &Env) -> bool {
        return self.next_transfer_ts <= env.ledger().timestamp() + SECONDS_IN_WEEK;
    }
}

pub fn calculate_next_claim(e: &Env, investment: &Investment) -> Claim {
    let next_claim = Claim {
        next_transfer_ts: match investment.last_transfer_ts {
            lts if lts > 0  => lts + SECONDS_IN_MONTH,
            _ => e.ledger().timestamp() + SECONDS_IN_MONTH
        },
        amount_to_pay: investment.regular_payment
    };

    next_claim
}