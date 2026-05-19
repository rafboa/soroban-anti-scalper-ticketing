# StellarPass: Anti-Scalper Ticketing System

> A decentralized platform on Stellar that makes ticket scalping **mathematically impossible** using Soroban smart contracts.

[![Build](https://img.shields.io/badge/build-passing-brightgreen)](.)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange)](https://www.rust-lang.org)
[![Network](https://img.shields.io/badge/network-Stellar%20Testnet-blue)](https://stellar.org)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](./LICENSE)

---

## Overview

StellarPass replaces static QR codes and PDFs with **smart-contract-governed ticket tokens** on the Stellar network. Every ticket is a ledger entry — not an image that can be screenshotted or a file that can be duplicated. Ownership, resale rights, and entry validation are all enforced cryptographically at the protocol level.

---

## Key Features

### 1. On-Chain Price Caps
Resale prices are capped at **Face Value + 10%** by the contract itself. The formula:

```
max_resale_price = face_value × 110 / 100
```

Any transfer attempt exceeding this ceiling is **rejected before a single token moves**. There is no human in the loop and no policy to circumvent.

### 2. Automated Royalties
Every secondary sale triggers an **instant 5% royalty** (500 basis points) routed directly to the event organiser's wallet — atomically, within the same transaction as the sale itself.

### 3. Dynamic Verification
Gate entry requires the ticket holder to **co-sign the check-in transaction** with their private key. Screenshots and printouts are cryptographically worthless. Once scanned, the ticket is permanently locked on-chain.

---

## Deployed Contract

| Network  | Contract ID                      |
|----------|----------------------------------|
| Testnet  | `CD642DOEAS62BGI7XMONDXLX5XSSKM6GPBJK22EO3RSHFW5UNW4VA6BA` |
| Mainnet  | `[Not yet deployed]`             |

**Testnet links:**
- [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CD642DOEAS62BGI7XMONDXLX5XSSKM6GPBJK22EO3RSHFW5UNW4VA6BA)
- [Open in Stellar Lab](https://lab.stellar.org/r/testnet/contract/CD642DOEAS62BGI7XMONDXLX5XSSKM6GPBJK22EO3RSHFW5UNW4VA6BA)

**Wasm hash:** `3e5d8721f15f235d7200b2ae680760ddb6018d4ba4a62bb422d25e46ae8e39f0`

### Contract Configuration

| Parameter              | Value                                                        |
|------------------------|--------------------------------------------------------------|
| Admin / Royalty Wallet | `GAG27SIQ3K3P7F46UNNSQUX5LSVRHRWEPVQS2CSKNBY73VBPACEVAIGI` |
| Face Value             | 100 stroops                                                  |
| Max Resale Multiplier  | 110% of face value (ceiling = 110 stroops)                   |
| Royalty                | 5% (500 basis points) per secondary sale                     |
| Network                | Stellar Testnet                                              |

- [View Admin Account on Stellar Expert](https://stellar.expert/explorer/testnet/account/GAG27SIQ3K3P7F46UNNSQUX5LSVRHRWEPVQS2CSKNBY73VBPACEVAIGI)

---

## Project Structure

```
.
├── contracts
│   └── ticketing
│       ├── src
│       │   ├── lib.rs        ← Core contract logic & price enforcement
│       │   └── test.rs       ← Full unit test suite
│       └── Cargo.toml        ← Contract crate manifest
├── frontend                  ← Next.js / React interface
│   ├── src
│   │   ├── app               ← Next.js App Router pages
│   │   ├── components        ← UI components (Gallery, Resale, CheckIn)
│   │   └── lib               ← ContractClient & wallet utilities
│   └── package.json
├── Cargo.toml                ← Workspace configuration
└── README.md
```

---

## Getting Started

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | stable ≥ 1.78 |
| `wasm32-unknown-unknown` target | via `rustup` |
| Soroban CLI | ≥ 21.x |
| Node.js | ≥ 18.x |

### Setup

```bash
# 1. Rust WASM target
rustup target add wasm32-unknown-unknown

# 2. Soroban CLI
cargo install --locked soroban-cli

# 3. Frontend dependencies
cd frontend && npm install
```

### Run Tests

```bash
# From the workspace root
cargo test
```

### Build Contract

```bash
cargo build --target wasm32-unknown-unknown --release

soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/stellarpass_ticketing.wasm
```

### Deploy to Testnet

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellarpass_ticketing.optimized.wasm \
  --source <YOUR_SECRET_KEY> \
  --network testnet
```

### Initialize

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET_KEY> \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --face_value 100 \
  --max_resale_multiplier 110 \
  --royalty_basis_points 500 \
  --royalty_recipient <ROYALTY_ADDRESS>
```

---

## Contract API

| Function | Signer | Description |
|----------|--------|-------------|
| `initialize(admin, face_value, max_resale_multiplier, royalty_basis_points, royalty_recipient)` | Admin | One-time event setup |
| `mint_ticket(ticket_id, to)` | Admin | Issue a new ticket token |
| `transfer_ticket(ticket_id, from, to, amount, token_addr)` | Seller | Transfer with price cap + royalty split |
| `check_in(ticket_id, owner)` | Ticket holder | Mark used at the gate — permanent |
| `get_ticket(ticket_id)` | — | Read-only state query |

---

## Security Model

| Threat | On-Chain Mitigation |
|--------|---------------------|
| Scalping / price gouging | Ceiling enforced in contract; over-limit transfers revert |
| Screenshot / printout fraud | Check-in requires holder's cryptographic signature |
| Malicious third-party check-in | `owner.require_auth()` on `check_in` |
| Re-entry with used ticket | `is_used = true` permanently blocks all further operations |
| Double-initialization | `AlreadyInitialized` guard on `initialize` |
| Integer overflow | `checked_mul` / `checked_div` on all price maths |

---

## License

MIT — see [LICENSE](./LICENSE).