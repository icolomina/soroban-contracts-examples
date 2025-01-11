use soroban_sdk::contracttype;

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

pub fn recalculate_contract_balances_from_amount(contract_balances: &mut ContractBalances, amounts: &Amount) {
    contract_balances.comission += amounts.amount_to_commission;
    contract_balances.reserve_fund += amounts.amount_to_reserve_fund;
    contract_balances.project += amounts.amount_to_invest;
}