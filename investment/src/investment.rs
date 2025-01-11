
use soroban_sdk::{contracttype, log, token::TokenClient, Address, Env};
use crate::{balance::{Amount, CalculateAmounts}, data::{ContractData, FromNumber}};

#[contracttype]
pub struct Investment {
    pub deposited: i128,
    pub accumulated_interests: i128,
    pub total: i128,
    pub claimable_ts: u64,
    pub last_transfer_ts: u64,
    pub status: InvestmentStatus,
    pub regular_payment: i128,
    pub paid: i128,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
#[contracttype]
pub enum InvestmentStatus {
    Blocked = 1,
    Claimable = 2,
    WaitingForPayment = 3,
    CashFlowing = 4,
    Finished = 5,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
#[contracttype]
pub enum InvestmentReturnType {
    ReverseLoan = 1,
    Coupon = 2,
    OneTimePayment = 3,
}

impl FromNumber for InvestmentReturnType {
    fn from_number<N>(value: N) -> Option<InvestmentReturnType> where N: Into<u32> {

        let value: u32 = value.into();
        match value {
            1 => Some(InvestmentReturnType::ReverseLoan),
            2 => Some(InvestmentReturnType::Coupon),
            3 => Some(InvestmentReturnType::OneTimePayment),
            _ => None,
        }
    }
}


pub fn build_investment(env: &Env, cd: &ContractData, amount: &i128 ) -> Investment{
    let amounts: Amount = Amount::from_investment(amount);
    let real_amount = amounts.amount_to_invest + amounts.amount_to_reserve_fund;
    let current_interest = (real_amount * (cd.interest_rate as i128 / 100)) / 100;
    let status: InvestmentStatus = match cd.claim_block_days {
        cbd if cbd > 0 => InvestmentStatus::Blocked,
        _ => InvestmentStatus::Claimable
    };

    let total = real_amount + current_interest;
    let claimable_ts = env.ledger().timestamp() + (cd.claim_block_days * 86400_u64);

    let regular_payment = match cd.return_type {
        InvestmentReturnType::Coupon => current_interest / cd.return_months as i128,
        InvestmentReturnType::ReverseLoan => total / cd.return_months as i128,
        InvestmentReturnType::OneTimePayment => 0
    };

    let investment = Investment {
        deposited: real_amount,
        accumulated_interests: current_interest,
        total,
        claimable_ts,
        last_transfer_ts: 0_u64,
        status,
        regular_payment,
        paid: 0_i128
    };

    investment
}

pub fn process_investment_claim(env: &Env, investment: &mut Investment, contract_data: &ContractData, tk: &TokenClient, addr: &Address) -> i128 {

    if investment.status == InvestmentStatus::Blocked {
        investment.status = InvestmentStatus::CashFlowing;
    }

    tk.transfer(&env.current_contract_address(), &addr, &investment.regular_payment);
    investment.paid += &investment.regular_payment;
    investment.last_transfer_ts = env.ledger().timestamp();
    log!(env, "fecha de ultima transferencia {}", investment.last_transfer_ts);
    
    if contract_data.return_type == InvestmentReturnType::ReverseLoan && investment.paid > (investment.total - investment.regular_payment) {
        investment.status = InvestmentStatus::Finished;
    }

    if contract_data.return_type == InvestmentReturnType::Coupon && investment.paid >= investment.accumulated_interests {
        tk.transfer(&env.current_contract_address(), &addr, &investment.deposited);
        investment.status = InvestmentStatus::Finished;
    }

    investment.regular_payment
}