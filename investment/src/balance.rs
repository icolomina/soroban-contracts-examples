use soroban_sdk::contracttype;

pub(self) const LOWER_AMOUNT_FOR_COMMISSION_REDUCTION: i128 = 100;
pub(self) const LOWER_DIVISOR: u32 = 10;
pub(self) const UPPER_DIVISOR: u32 = 60;
pub(self) const AMOUNT_PER_COMMISSION_REDUCTION: i128 = 400;

pub fn calculate_rate_denominator(amount: &i128) -> u32 {

    if amount <= &LOWER_AMOUNT_FOR_COMMISSION_REDUCTION {
        return LOWER_DIVISOR;
    }

    let a = (amount - LOWER_AMOUNT_FOR_COMMISSION_REDUCTION) / AMOUNT_PER_COMMISSION_REDUCTION;
    if a > UPPER_DIVISOR as i128 {
        return UPPER_DIVISOR;
    }

    LOWER_DIVISOR + a as u32
}

#[contracttype]
pub struct ContractBalances {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub received_so_far: i128,
    pub payments: i128,
    pub reserve_contributions: i128,
    pub project_withdrawals: i128,
    pub moved_from_project_to_reserve: i128
}

impl ContractBalances {
    pub fn new() -> Self {
        ContractBalances {
            reserve: 0_i128,
            project: 0_i128,
            comission: 0_i128,
            received_so_far: 0_i128,
            payments: 0_i128,
            reserve_contributions: 0_i128,
            project_withdrawals: 0_i128,
            moved_from_project_to_reserve: 0_i128
        }
    }

    pub fn sum(&self) -> i128 {
        return self.comission + self.project + self.reserve;
    }
}

#[contracttype]
pub struct Balance {
    pub deposited: i128,
    pub accumulated_interests: i128,
    pub total: i128,
}

pub struct Amount {
    pub amount_to_invest: i128,
    pub amount_to_reserve_fund: i128,
    pub amount_to_commission: i128
}

pub trait CalculateAmounts {
    fn from_investment(amount: &i128, i_rate: &u32) -> Amount;
}

impl CalculateAmounts for Amount {
    fn from_investment(amount: &i128, i_rate: &u32) -> Amount {

        let rate_denominator: u32 = calculate_rate_denominator(&amount);

        let amount_to_commission = amount * (*i_rate as i128) / (rate_denominator as i128) / 100 / 100;
        let amount_to_reserve_fund = amount * 5 / 100;
        let amount_to_invest = amount - amount_to_commission - amount_to_reserve_fund; 

        Amount {
            amount_to_invest,
            amount_to_reserve_fund,
            amount_to_commission,
        }
    }
}

pub fn recalculate_contract_balances_from_investment(contract_balances: &mut ContractBalances, amounts: &Amount) {
    contract_balances.comission += amounts.amount_to_commission;
    contract_balances.reserve += amounts.amount_to_reserve_fund;
    contract_balances.project += amounts.amount_to_invest;
    contract_balances.received_so_far += amounts.amount_to_reserve_fund + amounts.amount_to_invest;
}

pub fn increment_reserve_balance_from_company_contribution(contract_balances: &mut ContractBalances, amount: &i128) {
    contract_balances.reserve += amount;
    contract_balances.reserve_contributions += amount;
}

pub fn decrement_project_balance_from_company_withdrawal(contract_balances: &mut ContractBalances, amount: &i128) {
    contract_balances.project -= amount;
    contract_balances.project_withdrawals += amount;
}

pub fn decrement_project_balance_from_payment_to_investor(contract_balances: &mut ContractBalances, amount: &i128) {
    contract_balances.reserve -= amount;
    contract_balances.payments += amount;
}

pub fn move_from_project_balance_to_reserve_balance(contract_balances: &mut ContractBalances, amount: &i128) {
    contract_balances.project -= amount;
    contract_balances.reserve += amount;
    contract_balances.moved_from_project_to_reserve += amount;
}

