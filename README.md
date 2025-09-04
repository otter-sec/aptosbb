# AptosBB

Framework for writing vulnerability proof-of-concepts against Aptos mainnet smart contracts in an isolated environment.

## Overview

AptosBB creates a safe testing environment using the `FakeExecutor` with real mainnet state, allowing bug bounty hunters to test attack scenarios / develop proof-of-concepts against live contracts without putting at risk the actual contracts. To get started, you must only edit `src/pentest.rs`, where you have to add your own PoC targeting a specific protocol.

## Usage

### Default Mode (Rate Limited)
```bash
RUSTFLAGS="--cfg tokio_unstable" cargo run -- default
```
Uses anonymous connection to Aptos mainnet RPC. May hit rate limits with heavy usage.

### API Mode (Recommended) 
```bash
export APTOSBB_KEY=your_api_key_here
RUSTFLAGS="--cfg tokio_unstable" cargo run -- api
```
Uses authenticated connection with higher rate limits.

## Examples

The examples included in `src/pentest.rs` demonstrate several features of the framework:

### 1. Creating Attacker Accounts
```rust
// Create a new account with APT balance for testing
let attacker = bb.new_account();
println!("Attacker address: {}", attacker.address());

// Check account details and balance
if let Some(account_resource) = bb.read_account_resource_at_address(&attacker.address()) {
    println!("Sequence number: {}", account_resource.sequence_number());
}

if bb.has_apt_balance(&attacker) {
    if let Some(balance) = bb.read_apt_fungible_store_resource(&attacker) {
        println!("APT balance: {} (= {} APT)", balance, balance / 100_000_000);
    }
}
```

### 2. Publishing and Testing Custom Modules
```rust
// Deploy your own Aptos package
let hello_world_path = Path::new("./module");
let status = bb.publish_package(&attacker, hello_world_path);

// Call functions from your deployed module
let init_status = bb.run_entry_function(
    &attacker,
    hello_world_addr,
    "hello_world",
    "initialize",
    vec![],
    vec![],
);

// Read resources created by your module
let greeting_counter_tag = StructTag {
    address: hello_world_addr,
    module: Identifier::new("hello_world").unwrap(),
    name: Identifier::new("GreetingCounter").unwrap(),
    type_args: vec![],
};

if bb.exists_resource(&attacker.address(), greeting_counter_tag.clone()) {
    // Deserialize and inspect the resource data
}
```

### 3. Interacting with Live Mainnet Contracts
```rust
// Target real deployed contracts (example: ThalaSwap V1)
let thala_addr = AccountAddress::from_hex_literal("0x48271d39d0b05bd6efca2278f22277d6fcc375504f9839fd73f74ace240861af").unwrap();

// Prepare function arguments with proper type signatures
let type_args = vec![
    TypeTag::from_str("0x1::aptos_coin::AptosCoin").unwrap(),
    TypeTag::from_str("0x55987edfab9a57f69bac759674f139ae473b5e09a9283848c1f87faf6fc1e789::shrimp::ShrimpCoin").unwrap(),
    // ... more type arguments
];

let swap_args = vec![
    bcs::to_bytes(&amount_in).unwrap(),
    bcs::to_bytes(&min_amount_out).unwrap(),
];

// Execute the transaction and capture full output
let (swap_status, swap_output) = bb.run_transaction_with_output(&attacker, 
    TransactionPayload::EntryFunction(entry_fn));

// Analyze transaction effects
println!("Gas used: {}", swap_output.gas_used());
println!("Events emitted: {}", swap_output.events().len());

// Parse WriteSet to find newly created resources
let write_set_debug = format!("{:?}", swap_output.write_set());
```

### 4. Resource Analysis
```rust
// Read specific resources using the proper Aptos types
use aptos_types::account_config::fungible_store::FungibleStoreResource;
use aptos_types::account_config::ObjectGroupResource;

if let Some(store) = bb.executor.read_resource_from_group::<FungibleStoreResource>(
    &target_address, 
    &ObjectGroupResource::struct_tag()
) {
    println!("Balance: {} tokens", store.balance());
}
```

## License

Apache 2.0