use core::ops::Add;

use soroban_sdk::{contracterror, contracttype, log, vec, Address, Env, String, Vec};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub trait FromNumber {
    fn from_number<N>(number: N) -> Option<Self> 
    where 
        Self: Sized,
        N: Into<u32>;
}

#[contracttype]
#[derive(Copy, Clone)]
pub struct Claim {
    pub next_transfer_ts: u64,
    pub amount_to_pay: i128
}

impl Claim {
    pub fn is_claim_next(&self, env: &Env) -> bool {
        let week_seconds: u64 = 7 * 24 * 60 * 60;
        log!(env, "Next Transfer Ts es {}", self.next_transfer_ts);
        log!(env, "Ledger Timestamp is {}", env.ledger().timestamp());
        log!(env, "Seconds in a week is {}", week_seconds);
        return self.next_transfer_ts <= env.ledger().timestamp() + week_seconds;
    }
}

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
pub enum MultisigStatus {
    WaitingForSignatures = 1,
    Completed = 2,
}

#[contracttype]
pub struct ContractData {
    pub interest_rate: u32,
    pub claim_block_days: u64,
    pub token: Address,
    pub project_address: Address,
    pub admin: Address,
    pub state: State,
    pub return_type: InvestmentReturnType,
    pub return_months: u32,
    pub min_per_investment: i128,
    pub goal: i128,
}

#[contracttype]
pub struct ContractBalances {
    pub reserve_fund: i128,
    pub project: i128,
    pub comission: i128
}

impl ContractBalances {
    pub fn new() -> Self {
        ContractBalances {
            reserve_fund: 0_i128,
            project: 0_i128,
            comission: 0_i128
        }
    }

    pub fn sum(&self) -> i128 {
        return self.comission + self.project + self.reserve_fund;
    }
}

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

pub struct Amount {
    pub amount_to_invest: i128,
    pub amount_to_reserve_fund: i128,
    pub amount_to_commission: i128
}

pub trait CalculateAmounts {
    fn from_investment(amount: &i128) -> Amount;
}

impl CalculateAmounts for Amount {
    fn from_investment(amount: &i128) -> Amount {

        let amount_to_commission = amount * 2 / 100;
        let amount_to_reserve_fund = amount * 5 / 100;
        let amount_to_invest = amount - amount_to_commission - amount_to_reserve_fund; 

        Amount {
            amount_to_invest,
            amount_to_reserve_fund,
            amount_to_commission,
        }
    }
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

#[contracttype]
pub struct Balance {
    pub deposited: i128,
    pub accumulated_interests: i128,
    pub total: i128,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracterror]
pub enum Error {
    AddressInsufficientBalance = 1,
    ContractInsufficientBalance = 2,
    ContractNotInitialized = 3,
    AmountLessOrEqualThan0 = 4,
    AmountLessOrThan0 = 5,
    ContractAlreadyInitialized = 6,
    RateMustBeGreaterThan0 = 7,
    DepositTtlMustBeGreaterThan0 = 8,
    AddressNotClaimableYet = 9,
    AddressAlreadyDeposited = 10,
    ContractFinancingReached = 11,
    GoalMustBeGreaterThan0 = 12,
    UnsupportedReturnType = 13,
    AddressHasNotInvested = 14,
    AddressInvestmentIsNotClaimableYet = 15,
    AddressInvestmentIsFinished = 16,
    AddressInvestmentNextTransferNotClaimableYet = 17,
    AddressAlreadyInvested = 18,
    ContractHasReachedInvestmentGoal = 19,
    ProjectAddressInsufficientBalance = 20
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracttype]
pub enum State {
    Pending = 1,
    Initialized = 2,
    Deposited = 3,
    NoDeposited = 4,
    Withdrawn = 5,
    FinancingReached = 6,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    InterestRate,
    DepositStart(Address),
    Token,
    Admin,
    State,
    ClaimBlockDays,
    ClaimTime(Address),
    AddressStatus(Address),
    ContractData,
    Investment(Address),
    BalanceReserveFund,
    BalanceComission,
    BalanceProject,
    ClaimsMap,
    MultisigRequest,
    ContractBalances
}
