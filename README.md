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

## Project Vision

The live music and events industry loses billions every year to ticket scalping. Fans pay 3–10× face value. Artists and organisers see none of that money. Existing "anti-scalp" policies are enforced by humans and routinely circumvented by bots.

StellarPass's vision is to make scalping **not a policy problem but a physics problem** — something the math of the blockchain makes impossible, not just against the rules.

- **For fans**: guaranteed access to tickets at fair prices, with no fear of fraudulent or duplicated tickets
- **For artists and organisers**: automated royalty income on every secondary sale, instant settlement, no intermediaries
- **For the industry**: a trustless ticketing layer that any event platform can build on top of, without relying on centralised gatekeepers
- **Long term**: a world where your ticket lives in your wallet alongside your identity — transferable, verifiable, and always fairly priced, whether you're attending a local gig or a global stadium tour

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

| Function | Signer | Permission | Description |
|----------|--------|------------|-------------|
| `initialize(admin, face_value, max_resale_multiplier, royalty_basis_points, royalty_recipient)` | Admin | One-time only | Set up event config |
| `mint_ticket(ticket_id, to)` | Admin | Admin only | Issue a new ticket token |
| `transfer_ticket(ticket_id, from, to, amount, token_addr)` | Seller | Owner of ticket | Transfer with price cap + royalty split |
| `check_in(ticket_id, owner)` | Ticket holder | Owner of ticket | Mark used at the gate — permanent |
| `get_ticket(ticket_id)` | — | Public | Read-only state query |

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

## Test Coverage

The contract includes 7 hermetic unit tests using `soroban-sdk`'s mock environment — no live network required:

| # | Test | Validates |
|---|------|-----------|
| 1 | `test_secondary_transfer_legal_price_succeeds` | Correct balance splits for buyer, seller, and royalty wallet after a legal secondary sale |
| 2 | `test_scalper_blocked_price_above_ceiling` | Contract panics when resale price exceeds the 110% ceiling |
| 3 | `test_check_in_marks_ticket_as_used` | `is_used` is permanently set to `true` after gate scan |
| 4 | `test_used_ticket_cannot_be_transferred` | Transfer of a used ticket is rejected |
| 5 | `test_used_ticket_cannot_be_checked_in_again` | Double check-in on the same ticket is rejected |
| 6 | `test_double_initialize_panics` | Contract cannot be initialized a second time |
| 7 | `test_non_owner_cannot_transfer` / `test_duplicate_mint_panics` / `test_get_ticket_returns_none` | Edge cases: wrong owner, duplicate mint, unknown ticket ID |

---

## Future Scope

### Short-Term

1. **Event Metadata**: Add title, venue, date, and seat number fields to each ticket record
2. **Batch Minting**: Allow the admin to mint multiple tickets in a single transaction for efficiency
3. **Configurable Royalty per Event**: Support different royalty rates for different event types
4. **Ticket Expiry**: Auto-invalidate tickets after the event date using Soroban's ledger timestamp

### Medium-Term

5. **Multi-Event Support**: Deploy a single factory contract that manages multiple events independently
6. **Whitelist / KYC**: Restrict ticket purchases to pre-approved wallet addresses for high-demand events
7. **Refund Mechanism**: Allow the admin to trigger refunds before an event, returning tokens to buyers
8. **On-Chain Event Log**: Emit contract events for every mint, transfer, and check-in for real-time indexing

### Long-Term

9. **Frontend DApp**: Complete the Next.js interface with Freighter wallet integration, ticket gallery, and resale portal
10. **Mobile Wallet SDK**: Native iOS and Android SDKs so attendees can store and present tickets from their phone
11. **Cross-Contract Composability**: Allow DeFi protocols to use ticket ownership as collateral or for governance
12. **Zero-Knowledge Check-In**: Private gate entry where the holder proves ownership without revealing their wallet address
13. **DAO Governance**: Let ticket holders vote on event decisions (setlist, venue, charity donations) proportional to tickets held
14. **Mainnet Launch**: Deploy to Stellar mainnet with a real payment token (USDC or XLM) and partner with an event organiser

---

## License

MIT — see [LICENSE](./LICENSE).

---

**StellarPass** — Fair Tickets, On-Chain. No Bots. No Fraud. No Excuses. 🎟️✦