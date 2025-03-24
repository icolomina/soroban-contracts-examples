use soroban_sdk::{contracterror, contracttype, Address};
use crate::investment::InvestmentReturnType;

pub trait FromNumber {
    fn from_number<N>(number: N) -> Option<Self> 
    where 
        Self: Sized,
        N: Into<u32>;
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
    ProjectAddressInsufficientBalance = 20,
    WithdrawalUnexpectedSignature = 21,
    WithdrawalExpiredSignature = 22,
    WithdrawalInvalidAmount = 23
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
