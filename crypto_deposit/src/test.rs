#![cfg(test)]

use crate::{CryptoDeposit, CryptoDepositClient};
use soroban_sdk::{Env, testutils::Address as _, Address, token};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        TokenAdminClient::new(e, &sac.address()),
    )
}

fn create_contract<'a>(e: &'a Env, amount: &'a i128) -> (CryptoDepositClient<'a>, Address) {
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let (token, token_admin) = create_token_contract(&e, &admin);
    token_admin.mint(&user, &amount);

    let client = CryptoDepositClient::new(
        e,
        &e.register(
            CryptoDeposit {}, 
            (admin, token.address)
        )
    );

    (client, user.clone())
}

#[test]
fn test_deposit() {
    let e = Env::default();
    let test_data = create_contract(&e, &100_i128);
    assert_eq!(test_data.0.deposit(&test_data.1, &50), 50);
}
