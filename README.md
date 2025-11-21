# Soroban Contracts

This repository contains several smart contracts developed for the Soroban/Stellar platform. The following describes the functionality of each contract, how to build and test the contracts, and how to use `stellar-cli` to generate and deploy WASM code to Testnet.

> [!WARNING]
> These contracts are educational examples and have not been tested or audited. They are useful for learning and prototyping, but should not be used in production without professional auditing. Please refer to the license for more information.

## Contract Descriptions

### Ballot

Contract for managing voting processes. Allows storing users eligible to vote, registering votes and delegations, and controlling voting dates. Includes logic to verify if a user has voted, has delegated their vote, or has delegated votes. The application can interact with the contract to register votes and delegations without requiring direct user signature.

### Crypto Deposit

Contract for token deposits to the contract address. Allows initializing the contract with an administrator and a token, and performing user-authenticated deposits. The contract transfers deposited tokens to its own address and maintains an updated balance.

### Investment

Advanced contract for project investment management. Allows configuring parameters such as administrator, project address, token, interest rate, return type, return months, minimum per investment, etc. Includes functions for investing, claiming returns, multisig withdrawals, and balance and reserve control. Each contract manages the funds of a single project.

### HouseAsset

Contract representing a real estate asset (e.g., a house). Allows initializing the asset with an owner and identifier, approving transfers, and managing asset ownership and metadata.

### HousePurchase

Contract for managing property purchases between buyer and seller, using the `HouseAsset` contract as asset representation. Allows initializing the purchase, managing payments (first payment and remainder), and transferring asset ownership to the buyer once payments are completed.

---

## Build and Test Execution

To compile and test the contracts, make sure you have the Soroban environment configured following the [official documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup).

### Compile a contract

```bash
cargo build
```

### Run tests

```bash
cargo test
```

> Execute these commands inside the root folder of each contract (e.g., `soroban-contracts/ballot`).

---

## Using Stellar-CLI to Generate and Deploy WASM

### Generate WASM code

From the contract root folder, execute:

```bash
cargo build --target wasm32-unknown-unknown --release
```

The `.wasm` file will be generated in `target/wasm32-unknown-unknown/release/`.

### Deploy the contract to Testnet

1. Install `stellar-cli` following the [official guide](https://github.com/stellar/stellar-cli).
2. Authenticate your account and configure the Testnet network.
3. Deploy the contract:

```bash
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/<contract_name>.wasm --network testnet
```

4. Interact with the contract using `stellar-cli` commands to invoke functions, query state, etc.

---
