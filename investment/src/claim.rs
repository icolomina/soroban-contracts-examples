use soroban_sdk::{contracttype, Env};
use crate::investment::Investment;

#[contracttype]
#[derive(Copy, Clone)]
pub struct Claim {
    pub next_transfer_ts: u64,
    pub amount_to_pay: i128
}

impl Claim {
    pub fn is_claim_next(&self, env: &Env) -> bool {
        let week_seconds: u64 = 7 * 24 * 60 * 60;
        return self.next_transfer_ts <= env.ledger().timestamp() + week_seconds;
    }
}

pub fn calculate_next_claim(e: &Env, investment: &Investment) -> Claim {
    let seconds_in_a_month = 30 * 24 * 60 * 60;
    let next_claim = Claim {
        next_transfer_ts: match investment.last_transfer_ts {
            lts if lts > 0  => lts + seconds_in_a_month,
            _ => e.ledger().timestamp() + seconds_in_a_month
        },
        amount_to_pay: investment.regular_payment
    };

    next_claim
}