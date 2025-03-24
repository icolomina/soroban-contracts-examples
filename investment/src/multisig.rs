use soroban_sdk::{contracttype, String, Vec, Address, vec, Env};
use crate::data::ContractData;

#[contracttype]
pub struct MultisigRequest {
    pub function: String,
    pub successful_signatures: u32,
    pub expected_addrs: Vec<Address>,
    pub signed_addrs: Vec<Address>,
    pub valid_ts: u64, // Measured in ledger ts
    pub amount: i128
}

impl MultisigRequest {
    
    pub fn add_sig(&mut self, addr: Address) -> u32 {
        if !self.expected_addrs.contains(addr.clone()) {
            return self.signed_addrs.len()
        }

        self.signed_addrs.push_back(addr);
        self.signed_addrs.len()
    }

    pub fn is_valid_signature(&self, addr: Address) -> bool {
        self.expected_addrs.contains(addr)
    }

    pub fn is_completed(&self) -> bool {
        return self.signed_addrs.len() == self.expected_addrs.len();
    }

    pub fn is_expired(&self, current_ts: u64) -> bool {
        return current_ts > self.valid_ts
    }

    pub fn new(e: &Env, contract_data: &ContractData, function: String, successful_signatures: u32, amount: i128, valid_ts: u64) -> Self {

        let expected_addrs: Vec<Address> = vec![
            e,
            contract_data.admin.clone(),
            contract_data.project_address.clone()
        ];

        MultisigRequest {
            function,
            successful_signatures,
            expected_addrs,
            signed_addrs: Vec::new(e),   
            amount,
            valid_ts,
        }
    }
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MultisigStatus {
    WaitingForSignatures = 1,
    Completed = 2,
}