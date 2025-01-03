## Soroban contract examples

Here you can find soroban contract examples. I try to improve and update these contracts continuously with new features and feedback provided by other developers.

### Ballot without token
This contract manages a ballot process following a custodial approach. Allowed-to-vote users are stored in the contract storage. When a user wants to vote, He does not need to sign a transactión with his wallet but the application would be in charge of storing the vote in the contract. 

### Ballot
This contract also manages a ballot process but, in this case, the user must hold a token to be able to vote. The token is defined by the BallotToken contract (ballot/BallotToken). The user must sign the transaction with his wallet since authorization is required and, before storing the vote, the contract ensures the user address holds the token checking the balance. 

### House Purchase
This contract manages a house purchase between buyer and seller. It uses another contract which acts as an asset and represents the underlying asset, that is, the house. After the buyer send the payment to the current asset owner, it changes the ownership of the asset to the buyer.

### Investment
This Investment Contract provides a robust framework for managing investments on the Soroban platform. It includes features for investment management, return claims, and multisig withdrawals, ensuring secure and efficient handling of funds. This contract is designed to be installed for every single project so that each address manages a single project funds. After the contract is installed, the "init" function must be called. This function loads all the required data the contract needs to work:

- **admin_addr**: The contract administrator address. This address which administrates the contract
- **project_address**: The project address. This is the address to which the funds raised will be sent.
- **token_addr**: This is the address of the token that will be used to manage the project balance (for instance, USDC).
- **i_rate**: The interest rate the project offers to te users.
- **claim_block_days**: The number of days during which the investor must maintain his investment before claiming te gains.
- **goal**: The fundraising goal for the project.
- **return_type**: The way in which the investor will recover his investment (capital + interest)
   - **Reverse Loan**: The amount is returned to the user as a monthly payment during an established munber of months
   - **Coupon**: The interests are returned to the user as a monthly payment during an established number of months. After the last month, the user receives the capital too.
   - **One time payment**: The user receives the payment within a unique payment.
- **return_months**: The number of months established to return the payment.
- **min_per_investment**: The minimum amount per investment.

After the contract is initialized, users can send their their transfers and become investors, This is done calling the "invest" function. For each transfer, the contract reserves a 5% as a reserve fund and a 2% as a comission.
- The "claim" function is used to send the payments to the investors after the "claim_block_days" has passed. This function checks if its been a month since the last address payment and, if so, sends the payment and updates the last payment timestamp.
- The "project_withdrawn" function allows the project address to withdran funds. It requires both the project address and the admin address sign.
- The "check_project_address_balance" ensures that the contract has sufficient funds to cover payments for the next 7 days.
- The "stop_investments" flags the contract so that no more investors transfers will be accepted.
- The "get_contract_balance" function retunrs the current balance of the contract.

### Simple deposit
A contract to make a simple deposit to the contract address.

-----------------------------------------------------------------------------------

**IMPORTANT**: These contracts have a test suite but they have not been audited. They can serve as a base for learning but not for being used directly 
in a real application without being audited first.

You can read about these contracts in my dev.to blog:

- **Ballot**: https://dev.to/icolomina/building-a-ballot-contract-using-soroban-plataform-and-rust-sdk-1hg1
- **Ballot with token**: https://dev.to/icolomina/using-tokenization-to-control-a-soroban-voting-smart-contract-3lm6
- **House Purchase**: New version comming soon

> The House Purchase article link shows how to connect to the contract using PHP. It's also a good way to learn how the contract works

- **Paid Account**: Comming soon
- **Simple Deposit**: https://dev.to/icolomina/making-deposits-to-an-smart-contract-using-php-symfony-and-the-soroban-technology-4f10

> The Simple Deposit article link shows an explanation about the contract and how to interact with it using a PHP / Symfony application.

## Test the contracts

To test the contracts, you must prepare first your environment. Follow the [soroban official documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) to achieve it.
After having the environment ready, follow the next steps:

### Build the contract

```shell
cargo build
```

### Test the contract
```shell
cargo test
```

> The last commands must be executed inside the contract root folder. For instance: *soroban-contracts/paid_account*.
