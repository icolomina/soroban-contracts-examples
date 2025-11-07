use soroban_sdk::{contracterror, contracttype, symbol_short, Address, Symbol};
use crate::investment::InvestmentReturnType;

pub trait FromNumber {
    fn from_number<N>(number: N) -> Option<Self> 
    where 
        Self: Sized,
        N: Into<u32>;
}

pub const TOPIC_CONTRACT_BALANCE_UPDATED: Symbol = symbol_short!("CBUPDATED");
pub const TOPIC_CONTRACT_STATUS_UPDATED: Symbol = symbol_short!("STUPDATED");


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
    AmountLessOrEqualThan0 = 4,
    AddressHasNotInvested = 14,
    AddressInvestmentIsNotClaimableYet = 15,
    AddressInvestmentIsFinished = 16,
    AddressInvestmentNextTransferNotClaimableYet = 17,
    WithdrawalUnexpectedSignature = 21,
    WithdrawalExpiredSignature = 22,
    WithdrawalInvalidAmount = 23,
    ProjectBalanceInsufficientAmount = 24,
    ContractMustBePausedToRestartAgain = 25,
    ContractMustBeActiveToBePaused = 26,
    ContractMustBeActiveToInvest = 27,
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracttype]
pub enum State {
    Pending = 1,
    Actve = 2,
    FundsReached = 3,
    Paused = 4,
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
    ContractBalances,
    ContractFundsReceived
}
