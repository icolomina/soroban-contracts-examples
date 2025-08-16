#![cfg(test)]

use super::{Ballot, BallotClient};
use soroban_sdk::{Env, symbol_short, testutils::Address as _, Address};

#[test]
fn vote_test() {
    let env = Env::default();
    let client = create_client(&env);
    let addr_admin = Address::generate(&env);

    assert_eq!(client.vote(&addr_admin, &symbol_short!("hyyt76"), &symbol_short!("Laborist")), true);
    assert_eq!(client.vote(&addr_admin, &symbol_short!("ptft37"), &symbol_short!("Conserv")), true);
    assert_eq!(client.vote(&addr_admin, &symbol_short!("oo9gt6"), &symbol_short!("Conserv")), true);

    let count = client.count(&addr_admin);

    assert_eq!(count.get(symbol_short!("Laborist")).unwrap(), 1);
    assert_eq!(count.get(symbol_short!("Conserv")).unwrap(), 2);

    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("oonvv5"));
    assert_eq!(client.vote(&addr_admin, &symbol_short!("oonvv5"), &symbol_short!("Conserv")), true);

    let count = client.count(&addr_admin);

    assert_eq!(count.get(symbol_short!("Laborist")).unwrap(), 1);
    assert_eq!(count.get(symbol_short!("Conserv")).unwrap(),4);

}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn vote_out_of_dates_test() {
    let env = Env::default();
    let client = create_client(&env);
    let addr_admin = Address::generate(&env);

    let ts_start: u64 = 1689238800; // 2023-07-13 09:00:00
    let ts_end: u64 = 1689551999; // 2023-07-16 23:59:59

    client.configure(&addr_admin, &ts_start, &ts_end);
    client.vote(&addr_admin, &symbol_short!("hyyt76"), &symbol_short!("Laborist"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn vote_test_already_voted() {
    let env = Env::default();
    let client = create_client(&env);
    let addr_admin = Address::generate(&env);

    client.vote(&addr_admin, &symbol_short!("hyyt76"), &symbol_short!("Laborist"));
    client.vote(&addr_admin, &symbol_short!("hyyt76"), &symbol_short!("Laborist"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn vote_test_delegated_vote() {
    let env = Env::default();
    let client = create_client(&env);
    let addr_admin = Address::generate(&env);

    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("oonvv5"));
    client.vote(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("Laborist"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn delegate_test_has_delegated_votes() {
    let env = Env::default();
    let client = create_client(&env);

    let addr_admin = Address::generate(&env);
    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("oonvv5"));
    client.delegate(&addr_admin, &symbol_short!("oonvv5"), &symbol_short!("ppky55"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn delegate_test_has_delegated_his_vote() {
    let env = Env::default();
    let client = create_client(&env);

    let addr_admin = Address::generate(&env);
    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("oonvv5"));
    client.delegate(&addr_admin, &symbol_short!("hhvftp"), &symbol_short!("ippcxs"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn delegate_test_o_voter_has_voted() {
    let env = Env::default();
    let client = create_client(&env);

    let addr_admin = Address::generate(&env);
    client.vote(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("Laborist"));
    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("hhcfrp"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn delegate_test_d_voter_has_voted() {
    let env = Env::default();
    let client = create_client(&env);

    let addr_admin = Address::generate(&env);
    client.vote(&addr_admin, &symbol_short!("hhcfrp"), &symbol_short!("Laborist"));
    client.delegate(&addr_admin, &symbol_short!("ippcxs"), &symbol_short!("hhcfrp"));
}

fn create_client(env: &Env) -> BallotClient<'_>{
    env.mock_all_auths();

    let contract_id = env.register(Ballot, ());
    let client = BallotClient::new(&env, &contract_id);
    client
}