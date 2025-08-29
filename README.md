# AptosBB

Framework for penetration testing Aptos smart contracts against live mainnet state in an isolated environment.

## Overview

AptosBB creates a safe testing environment using Aptos `FakeExecutor` with real mainnet state, allowing bug bounty hunters to test attack scenarios / developing proof-of-concepts against live contracts without affecting the actual contracts.

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
Uses authenticated connection with higher rate limits. Get your API key from [https://geomi.dev/](https://geomi.dev/).

## How it works

Only edit `src/pentest.rs` to add your own test scenarios targeting specific contracts and functions.

## License

Apache 2.0