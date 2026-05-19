//! # StellarPass — Unit Test Suite
//!
//! Uses `soroban-sdk`'s built-in mock environment so every test is
//! hermetic and requires no live network.
//!
//! ## Test matrix
//! | # | Scenario                       | Expected outcome             |
//! |---|-------------------------------|------------------------------|
//! | 1 | Happy-path secondary transfer  | Balances correct, owner updated |
//! | 2 | Scalper blocked                | Contract panics (PriceCeilingExceeded) |
//! | 3 | Check-in flow                  | Ticket marked used; transfer blocked |
//! | 4 | Double initialisation          | Panics (AlreadyInitialized)  |
//! | 5 | Wrong owner transfer           | Panics (NotTicketOwner)      |
//! | 6 | Duplicate mint                 | Panics (TicketAlreadyExists) |

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env,
};

use crate::{StellarPassContract, StellarPassContractClient};

// ─────────────────────────────────────────────────────────────────────────────
// Shared test fixture
// ─────────────────────────────────────────────────────────────────────────────

struct Fixture {
    env:              Env,
    contract_id:      Address,
    token_id:         Address,
    admin:            Address,
    royalty_wallet:   Address,
}

impl Fixture {
    /// Bootstrap a fresh environment with mocked auth, a token contract,
    /// and an initialised StellarPass contract.
    ///
    /// * face_value             = 100
    /// * max_resale_multiplier  = 110  (ceiling = 110 units)
    /// * royalty_basis_points   = 500  (5 %)
    fn new() -> Self {
        let env   = Env::default();
        env.mock_all_auths();

        let admin          = Address::generate(&env);
        let royalty_wallet = Address::generate(&env);

        // Deploy a mock Stellar Asset token.
        let token_asset    = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id       = token_asset.address();

        // Deploy & initialise the ticketing contract.
        let contract_id = env.register_contract(None, StellarPassContract);
        let client      = StellarPassContractClient::new(&env, &contract_id);

        client.initialize(
            &admin,
            &100_i128,
            &110_u32,
            &500_u32,
            &royalty_wallet,
        );

        Fixture { env, contract_id, token_id, admin, royalty_wallet }
    }

    fn client(&self) -> StellarPassContractClient {
        StellarPassContractClient::new(&self.env, &self.contract_id)
    }

    fn token(&self) -> TokenClient {
        TokenClient::new(&self.env, &self.token_id)
    }

    fn token_admin(&self) -> StellarAssetClient {
        StellarAssetClient::new(&self.env, &self.token_id)
    }

    /// Mint `amount` units of the test token to `recipient`.
    fn fund(&self, recipient: &Address, amount: i128) {
        self.token_admin().mint(recipient, &amount);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — Happy-path secondary transfer
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_secondary_transfer_legal_price_succeeds() {
    let f = Fixture::new();

    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    // Fund Fan B so they can pay.
    f.fund(&fan_b, 500);

    // Admin mints ticket #1 to Fan A (simulates a primary-sale outcome).
    f.client().mint_ticket(&1_u32, &fan_a);

    // Fan A sells to Fan B at 105 — within the 110 ceiling.
    let sale_price = 105_i128;
    f.client().transfer_ticket(
        &1_u32,
        &fan_a,
        &fan_b,
        &sale_price,
        &f.token_id,
    );

    // Ownership transferred.
    let record = f.client().get_ticket(&1_u32).expect("ticket must exist");
    assert_eq!(record.owner, fan_b, "Fan B should now own the ticket");
    assert!(!record.is_used, "ticket should not be used yet");

    // Royalty = 5 % of 105 = 5 (integer division).
    let expected_royalty:   i128 = 105 * 500 / 10_000; // 5
    let expected_proceeds:  i128 = sale_price - expected_royalty; // 100

    assert_eq!(
        f.token().balance(&f.royalty_wallet),
        expected_royalty,
        "royalty wallet should have received 5 % of the sale price",
    );
    assert_eq!(
        f.token().balance(&fan_a),
        expected_proceeds,
        "Fan A (seller) should have received sale proceeds minus royalty",
    );
    assert_eq!(
        f.token().balance(&fan_b),
        500 - sale_price,
        "Fan B (buyer) should have paid the full sale price",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — Scalper blocked: price exceeds ceiling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_scalper_blocked_price_above_ceiling() {
    let f = Fixture::new();

    let fan_b = Address::generate(&f.env);
    let fan_c = Address::generate(&f.env);

    f.fund(&fan_c, 1_000);

    // Admin mints to Fan B.
    f.client().mint_ticket(&2_u32, &fan_b);

    // Fan B attempts to sell at 200 — double face value, far above the 110 ceiling.
    // This MUST panic with PriceCeilingExceeded.
    f.client().transfer_ticket(
        &2_u32,
        &fan_b,
        &fan_c,
        &200_i128,      // 200 > 110 ceiling
        &f.token_id,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — Check-in marks ticket used; subsequent transfer panics
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_check_in_marks_ticket_as_used() {
    let f       = Fixture::new();
    let fan_a   = Address::generate(&f.env);

    f.client().mint_ticket(&3_u32, &fan_a);
    f.client().check_in(&3_u32, &fan_a);

    let record = f.client().get_ticket(&3_u32).expect("ticket must exist");
    assert!(record.is_used, "ticket should be permanently marked as used");
}

#[test]
#[should_panic]
fn test_used_ticket_cannot_be_transferred() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);

    f.client().mint_ticket(&4_u32, &fan_a);
    f.client().check_in(&4_u32, &fan_a);

    // Transfer of a used ticket MUST panic with TicketAlreadyUsed.
    f.client().transfer_ticket(
        &4_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );
}

#[test]
#[should_panic]
fn test_used_ticket_cannot_be_checked_in_again() {
    let f   = Fixture::new();
    let fan = Address::generate(&f.env);

    f.client().mint_ticket(&5_u32, &fan);
    f.client().check_in(&5_u32, &fan);

    // Second check-in MUST panic with TicketAlreadyUsed.
    f.client().check_in(&5_u32, &fan);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — Double initialisation is blocked
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_double_initialize_panics() {
    let f = Fixture::new(); // first init already done

    // Second init MUST panic with AlreadyInitialized.
    f.client().initialize(
        &f.admin,
        &100_i128,
        &110_u32,
        &500_u32,
        &f.royalty_wallet,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — Wrong owner cannot transfer
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_non_owner_cannot_transfer() {
    let f         = Fixture::new();
    let real_owner = Address::generate(&f.env);
    let impostor   = Address::generate(&f.env);
    let buyer      = Address::generate(&f.env);

    f.fund(&buyer, 200);
    f.client().mint_ticket(&6_u32, &real_owner);

    // Impostor tries to sell a ticket they do not own — MUST panic.
    f.client().transfer_ticket(
        &6_u32,
        &impostor,   // not the owner
        &buyer,
        &100_i128,
        &f.token_id,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6 — Duplicate mint is blocked
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_duplicate_mint_panics() {
    let f  = Fixture::new();
    let to = Address::generate(&f.env);

    f.client().mint_ticket(&7_u32, &to);
    // Second mint with the same ID MUST panic with TicketAlreadyExists.
    f.client().mint_ticket(&7_u32, &to);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7 — get_ticket returns None for non-existent ticket
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_get_ticket_returns_none_for_unknown_id() {
    let f = Fixture::new();
    assert!(
        f.client().get_ticket(&999_u32).is_none(),
        "get_ticket should return None for an unminted ticket ID",
    );
}