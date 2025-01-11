#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, symbol_short};

pub const ADMIN: Symbol = symbol_short!("admin");
pub const TOKEN: Symbol = symbol_short!("token");

#[contract]
pub struct CryptoDeposit;

#[contractimpl]
impl CryptoDeposit {

    pub fn __constructor(env: Env, admin_addr: Address, token_addr: Address) {
        env.storage().instance().set(&ADMIN, &admin_addr);
        env.storage().instance().set(&TOKEN, &token_addr);
    }
    
    
    pub fn deposit(env: Env, addr: Address, amount: i128) -> i128 {

        addr.require_auth();
        let token: Address = env.storage().instance().get(&TOKEN).unwrap();
        
        let tk = token::Client::new(&env, &token);
        tk.transfer(&addr, &env.current_contract_address(), &amount);
        let current_contract_balance = tk.balance(&env.current_contract_address());
        current_contract_balance
    }
}

mod test;

