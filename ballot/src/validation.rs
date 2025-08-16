use soroban_sdk::Env;

pub fn is_valid_date(env: &Env, start: &u64, end: &u64) -> bool {
    let current_timestamp = env.ledger().timestamp();
    current_timestamp >= *start && current_timestamp <= *end
}