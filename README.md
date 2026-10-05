# StellarPass 🎟️

> **Fair tickets, on-chain. No scalpers. No bots. No fraud.**  
> A decentralized ticketing factory built on **Stellar Soroban** that makes ticket scalping, bot hoarding, and counterfeit transfers mathematically impossible.

[![Rust](https://img.shields.io/badge/Rust-1.78%2B-orange?logo=rust)](https://www.rust-lang.org)
[![Soroban](https://img.shields.io/badge/Soroban-Smart%20Contracts-7B42BC)](https://stellar.org)
[![Next.js](https://img.shields.io/badge/Next.js-16%20(Turbopack)-black?logo=next.js)](https://nextjs.org)
[![Stellar](https://img.shields.io/badge/Network-Stellar%20Testnet-08B5E5?logo=stellar)](https://stellar.org)
[![License](https://img.shields.io/badge/License-MIT-green)](./LICENSE)

---

## 💡 What is StellarPass?

The live entertainment and sports industries lose billions each year to bot syndicates and secondary ticket gouging. Fans pay 3–10× face value, artists and event organizers receive none of the markup, and duplicate-ticket fraud remains pervasive.

**StellarPass replaces vulnerable static QR codes and PDFs with smart-contract-governed ticket tokens.** 

Every ticket is an immutable, verifiable ledger record on Stellar. Resale price ceilings, attendee holding limits, anti-bot cooldowns, and cryptographic gate entry are enforced directly by smart contract code—not by discretionary human policies or bypassable CAPTCHAs.

---

## ✨ Key Features

| Feature | How It Works |
|---|---|
| **🛡️ Hard Resale Price Cap** | The smart contract strictly enforces `max_allowed = face_value × 110%`. Resale attempts above the ceiling automatically revert. |
| **🤖 Anti-Hoarding Rate Limiter** | Caps the maximum tickets a single wallet can acquire per event (`max_tickets_per_wallet`), preventing scalper bots from cornering inventory. |
| **⏱️ Anti-Flipping Cooldown** | Enforces a minimum cooldown period (`transfer_cooldown_seconds`) between ticket resales to stop high-frequency arbitrage bots. |
| **💰 Automated Organizer Royalties** | Secondary sales atomically split proceeds: organizers receive their configured royalty (up to 50%) instantly in the same transaction. |
| **🔒 Payment Token Verification** | Events bind their official payment token (e.g. USDC). Transfers with counterfeit tokens are automatically rejected. |
| **🎫 Dynamic Gate Check-In** | Dynamic QR codes expire every 60 seconds and require wallet signature verification. Screenshots and printouts are useless. |
| **🏭 Multi-Event Factory** | A single contract deployment can create and manage thousands of independent events. |
| **🚨 Emergency Circuit Breaker** | Organizers can instantly pause an event's operations in case of venue emergencies or suspected attacks. |

---

## 📁 Repository Structure

```text
├── contracts/ticketing/      # Soroban Smart Contract (Rust)
│   ├── src/lib.rs            # Factory contract, rate limits, and transfer logic
│   └── src/test.rs           # 18 comprehensive unit tests
├── frontend/                 # Next.js Web Application
│   ├── src/app/              # Next.js App Router (pages & global styling)
│   ├── src/components/       # UI Components (TicketGallery, ResalePortal, CheckInQR)
│   └── src/lib/              # Soroban ContractClient & Freighter wallet hook
├── Cargo.toml                # Rust workspace configuration
└── README.md                 # Public documentation
```

---

## 🚀 Quick Start Guide

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (stable ≥ 1.78)
- `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- [Node.js](https://nodejs.org) (v18+)
- [Soroban CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) (`cargo install --locked soroban-cli`)
- [Freighter Wallet](https://www.freighter.app/) browser extension

---

### 1. Test & Build the Smart Contract

Clone the repository and run the test suite:

```bash
# Run all 18 smart contract tests
cargo test
```

Build the optimized WebAssembly binary:

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled contract binary will be generated at:  
`target/wasm32-unknown-unknown/release/stellarpass_ticketing.wasm`

---

### 2. Deploy to Stellar Testnet (Optional)

Deploy the compiled Wasm binary to Stellar Testnet:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellarpass_ticketing.wasm \
  --source <YOUR_STELLAR_SECRET_KEY> \
  --network testnet
```

Initialize your event using the factory contract:

```bash
soroban contract invoke \
  --id <YOUR_CONTRACT_ID> \
  --source <YOUR_STELLAR_SECRET_KEY> \
  --network testnet \
  -- create_event \
  --event_id 1 \
  --admin <YOUR_ADMIN_PUBLIC_KEY> \
  --payment_token <PAYMENT_TOKEN_CONTRACT_ID> \
  --face_value 100 \
  --max_resale_multiplier 110 \
  --royalty_basis_points 500 \
  --royalty_recipient <YOUR_ROYALTY_WALLET> \
  --max_supply 1000 \
  --max_tickets_per_wallet 4 \
  --transfer_cooldown_seconds 60
```

---

### 3. Launch the Frontend DApp

1. Navigate to the frontend directory:
   ```bash
   cd frontend
   npm install
   ```

2. Set up your local environment file:
   ```bash
   cp .env.example .env.local
   ```
   Open `.env.local` and add your deployed contract and token IDs.

3. Start the Next.js development server:
   ```bash
   npm run dev
   ```

4. Open [http://localhost:3000](http://localhost:3000) in your browser, connect Freighter on Testnet, and start managing tickets!

---

## 📖 Smart Contract API

| Function | Access | Description |
|---|---|---|
| `create_event(...)` | Admin | Creates a new event with custom pricing, capacity, and rate limits |
| `mint_ticket(...)` | Admin | Mints an individual ticket with title, venue, date, and seat metadata |
| `batch_mint_tickets(...)` | Admin | Efficiently mints a batch of tickets in a single transaction |
| `transfer_ticket(...)` | Buyer & Seller | Dual-auth atomic transfer enforcing price caps, quotas, and cooldowns |
| `check_in(...)` | Ticket Holder | Validates and permanently marks ticket as used at gate entry |
| `refund_ticket(...)` | Admin | Returns face value to the ticket owner and invalidates the ticket |
| `set_paused(...)` | Admin | Circuit breaker to freeze or resume event operations |
| `set_whitelist_enabled(...)` | Admin | Enables or disables KYC/whitelist requirement for purchases |
| `add_to_whitelist(...)` | Admin | Adds an approved wallet address to the event whitelist |
| `get_ticket(...)` | Public | Fetches on-chain ticket status, ownership, and metadata |
| `get_event(...)` | Public | Fetches event configuration, pricing, and supply metrics |
| `get_user_ticket_balance(...)` | Public | Returns the number of tickets held by a wallet for quota checks |

---

## 🧪 Unit Test Coverage

StellarPass includes **18 unit tests** validating protocol security and edge cases:

* ✅ Legal secondary transfer with automatic royalty distribution
* ✅ Rejection of scalper price gouging above ceiling (reverts)
* ✅ Gate entry validation and permanent one-time usage lock
* ✅ Rejection of transfers on used tickets (reverts)
* ✅ Prevention of duplicate event initialization (reverts)
* ✅ Automatic invalidation of expired tickets based on ledger timestamp (reverts)
* ✅ Anti-hoarding quota enforcement (`max_tickets_per_wallet`) (reverts)
* ✅ Anti-flipping velocity cooldown enforcement (reverts)
* ✅ Successful transfer after cooldown expires
* ✅ Prevention of payment token spoofing (reverts)
* ✅ Hard supply ceiling enforcement (`max_supply`) (reverts)
* ✅ Emergency circuit breaker freeze (`set_paused`) (reverts)
* ✅ Organizer refund mechanism with balance tracking
* ✅ Rejection of transfers on refunded tickets (reverts)
* ✅ Multi-ticket batch minting functionality
* ✅ Whitelist restriction enforcement (reverts)
* ✅ Whitelist successful authorization
* ✅ Parameter validation guardrails on event creation (reverts)

---

## 📄 License

This project is open-source under the [MIT License](./LICENSE).

---

**StellarPass** — Built on Stellar. Fair by design. ✦