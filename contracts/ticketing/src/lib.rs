//! # StellarPass — Anti-Scalper Ticketing Contract
//!
//! Each ticket is a persistent ledger entry governed by hard pricing rules:
//!
//! * **Price ceiling**: resale ≤ face_value × max_resale_multiplier / 100
//!   (default 110 % → Face Value + 10 %)
//! * **Royalty split**: on secondary sales, royalty_basis_points / 10 000
//!   goes to `royalty_recipient`; the remainder goes to the seller.
//! * **Check-in lock**: `check_in` requires the holder's signature and
//!   permanently sets `is_used = true`, blocking every future operation.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror,
    token::Client as TokenClient,
    Address, Env,
};

// ─────────────────────────────────────────────────────────────────────────────
// Storage keys
// ─────────────────────────────────────────────────────────────────────────────

/// All storage keys used by the contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Singleton global configuration (instance storage).
    Config,
    /// Per-ticket record keyed by ticket ID (persistent storage).
    Ticket(u32),
}

// ─────────────────────────────────────────────────────────────────────────────
// Data structures
// ─────────────────────────────────────────────────────────────────────────────

/// Global event configuration — stored once in instance storage.
#[contracttype]
#[derive(Clone)]
pub struct Config {
    /// Event organiser address; the only account that may mint tickets.
    pub admin: Address,
    /// Original ticket price in token units (e.g. stroops).
    pub face_value: i128,
    /// Ceiling expressed as a percentage of face_value.
    /// 110 means a £100 ticket may not be resold above £110.
    pub max_resale_multiplier: u32,
    /// Secondary-market royalty in basis points (10 000 bp = 100 %).
    /// 500 bp = 5 %.
    pub royalty_basis_points: u32,
    /// Wallet that receives royalty payments on every secondary sale.
    pub royalty_recipient: Address,
}

/// State record for a single ticket — stored per-ticket in persistent storage.
#[contracttype]
#[derive(Clone)]
pub struct TicketRecord {
    /// Current registered owner.
    pub owner: Address,
    /// `true` once the ticket has been checked in at the gate.
    /// Any operation on a used ticket panics.
    pub is_used: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Error codes
// ─────────────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called a second time.
    AlreadyInitialized    = 1,
    /// Contract has not been initialised yet.
    NotInitialized        = 2,
    /// A ticket with this ID already exists in storage.
    TicketAlreadyExists   = 3,
    /// No ticket found for the given ID.
    TicketNotFound        = 4,
    /// The ticket has already been used (scanned at the gate).
    TicketAlreadyUsed     = 5,
    /// Caller is not the registered owner of the ticket.
    NotTicketOwner        = 6,
    /// Requested transfer price exceeds the resale price ceiling.
    PriceCeilingExceeded  = 7,
    /// A numeric overflow was detected in a price calculation.
    ArithmeticOverflow    = 8,
}

// ─────────────────────────────────────────────────────────────────────────────
// TTL ledger bump constants  (~30 days for config, ~1 year for tickets)
// ─────────────────────────────────────────────────────────────────────────────

const INSTANCE_TTL_LOW:  u32 = 17_280;   // threshold before bump
const INSTANCE_TTL_HIGH: u32 = 17_280;   // bump amount

const TICKET_TTL_LOW:    u32 = 518_400;  // threshold before bump
const TICKET_TTL_HIGH:   u32 = 518_400;  // bump amount

// ─────────────────────────────────────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct StellarPassContract;

#[contractimpl]
impl StellarPassContract {
    // ─────────────────────────────────────────────────────────────────────────
    // initialize
    // ─────────────────────────────────────────────────────────────────────────

    /// Set up the event contract exactly once.
    ///
    /// # Arguments
    /// * `admin`                 — Event organiser; only account that can mint.
    /// * `face_value`            — Original ticket price in token units.
    /// * `max_resale_multiplier` — e.g. 110 for a 110 % price ceiling.
    /// * `royalty_basis_points`  — e.g. 500 for 5 % royalty on secondary sales.
    /// * `royalty_recipient`     — Wallet that receives royalties.
    pub fn initialize(
        env:                    Env,
        admin:                  Address,
        face_value:             i128,
        max_resale_multiplier:  u32,
        royalty_basis_points:   u32,
        royalty_recipient:      Address,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(
            &DataKey::Config,
            &Config { admin, face_value, max_resale_multiplier, royalty_basis_points, royalty_recipient },
        );
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_LOW, INSTANCE_TTL_HIGH);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // mint_ticket
    // ─────────────────────────────────────────────────────────────────────────

    /// Issue a new ticket to `to`.  Only the admin may call this.
    ///
    /// Panics if a ticket with `ticket_id` already exists.
    pub fn mint_ticket(env: Env, ticket_id: u32, to: Address) {
        let config = Self::require_config(&env);
        config.admin.require_auth();

        let key = DataKey::Ticket(ticket_id);
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, Error::TicketAlreadyExists);
        }

        env.storage()
            .persistent()
            .set(&key, &TicketRecord { owner: to, is_used: false });
        env.storage()
            .persistent()
            .extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // transfer_ticket
    // ─────────────────────────────────────────────────────────────────────────

    /// Transfer ticket ownership from `from` to `to`, enforcing the resale
    /// price ceiling and routing royalties on secondary sales.
    ///
    /// The buyer (`to`) must have pre-approved `token_addr` to spend at least
    /// `amount` on behalf of this contract (standard SEP-41 allowance flow).
    ///
    /// # Payment routing
    /// * **Primary sale** (`from == admin`): full `amount` → admin.
    /// * **Secondary sale**: royalty → `royalty_recipient`; remainder → `from`.
    pub fn transfer_ticket(
        env:          Env,
        ticket_id:    u32,
        from:         Address,
        to:           Address,
        amount:       i128,
        token_addr:   Address,
    ) {
        // Seller must authorise the transfer.
        from.require_auth();

        let config = Self::require_config(&env);
        let key    = DataKey::Ticket(ticket_id);
        let mut record: TicketRecord = Self::require_ticket(&env, ticket_id);

        // Only the registered owner may initiate a transfer.
        if record.owner != from {
            panic_with_error!(&env, Error::NotTicketOwner);
        }

        // Used tickets cannot be resold.
        if record.is_used {
            panic_with_error!(&env, Error::TicketAlreadyUsed);
        }

        // ── Price ceiling ─────────────────────────────────────────────────────
        // max_allowed = face_value × max_resale_multiplier / 100
        let max_allowed: i128 = config.face_value
            .checked_mul(config.max_resale_multiplier as i128)
            .and_then(|v| v.checked_div(100))
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

        if amount > max_allowed {
            panic_with_error!(&env, Error::PriceCeilingExceeded);
        }

        // ── Payment routing ───────────────────────────────────────────────────
        let token    = TokenClient::new(&env, &token_addr);
        let this     = env.current_contract_address();

        if from == config.admin {
            // Primary sale: 100 % of the payment goes directly to the admin.
            token.transfer(&to, &config.admin, &amount);
        } else {
            // Secondary sale: split into royalty + seller proceeds.
            let royalty: i128 = amount
                .checked_mul(config.royalty_basis_points as i128)
                .and_then(|v| v.checked_div(10_000))
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

            let seller_proceeds: i128 = amount
                .checked_sub(royalty)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

            // Pull full payment into the contract first (single buyer approval).
            token.transfer(&to, &this, &amount);
            // Route royalty to the organiser.
            token.transfer(&this, &config.royalty_recipient, &royalty);
            // Route remainder to the seller.
            token.transfer(&this, &from, &seller_proceeds);
        }

        // ── Update ownership ──────────────────────────────────────────────────
        record.owner = to;
        env.storage().persistent().set(&key, &record);
        env.storage()
            .persistent()
            .extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // check_in
    // ─────────────────────────────────────────────────────────────────────────

    /// Scan a ticket at the gate.
    ///
    /// The ticket holder's **wallet signature is required** — the gate app
    /// cannot check in a ticket on the holder's behalf without their key.
    /// Once called, `is_used` is set to `true` and the ticket is permanently
    /// locked from transfers or further check-ins.
    pub fn check_in(env: Env, ticket_id: u32, owner: Address) {
        owner.require_auth();

        let key        = DataKey::Ticket(ticket_id);
        let mut record = Self::require_ticket(&env, ticket_id);

        if record.owner != owner {
            panic_with_error!(&env, Error::NotTicketOwner);
        }
        if record.is_used {
            panic_with_error!(&env, Error::TicketAlreadyUsed);
        }

        record.is_used = true;
        env.storage().persistent().set(&key, &record);
        env.storage()
            .persistent()
            .extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // get_ticket  (read-only)
    // ─────────────────────────────────────────────────────────────────────────

    /// Return the current record for a ticket, or `None` if it does not exist.
    /// Used by front-end wallets and off-chain verification engines.
    pub fn get_ticket(env: Env, ticket_id: u32) -> Option<TicketRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Ticket(ticket_id))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private helpers
    // ─────────────────────────────────────────────────────────────────────────

    fn require_config(env: &Env) -> Config {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
    }

    fn require_ticket(env: &Env, ticket_id: u32) -> TicketRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Ticket(ticket_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::TicketNotFound))
    }
}

mod test;