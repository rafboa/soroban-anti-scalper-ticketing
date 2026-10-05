#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String, Vec,
};

use crate::{StellarPassContract, StellarPassContractClient};

struct Fixture {
    env:            Env,
    contract_id:    Address,
    token_id:       Address,
    admin:          Address,
    royalty_wallet: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin          = Address::generate(&env);
        let royalty_wallet = Address::generate(&env);

        let token_asset = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id    = token_asset.address();

        let contract_id = env.register_contract(None, StellarPassContract);
        let client      = StellarPassContractClient::new(&env, &contract_id);

        client.create_event(
            &1_u32,
            &admin,
            &token_id,
            &100_i128,
            &110_u32,
            &500_u32,
            &royalty_wallet,
            &1_000_u32, // max_supply
            &4_u32,     // max_tickets_per_wallet
            &60_u64,    // transfer_cooldown_seconds
        );

        Fixture { env, contract_id, token_id, admin, royalty_wallet }
    }

    fn client(&self) -> StellarPassContractClient<'_> {
        StellarPassContractClient::new(&self.env, &self.contract_id)
    }

    fn token(&self) -> TokenClient<'_> {
        TokenClient::new(&self.env, &self.token_id)
    }

    fn token_admin(&self) -> StellarAssetClient<'_> {
        StellarAssetClient::new(&self.env, &self.token_id)
    }

    fn fund(&self, recipient: &Address, amount: i128) {
        self.token_admin().mint(recipient, &amount);
    }

    fn mint(&self, ticket_id: u32, to: &Address) {
        self.client().mint_ticket(
            &1_u32,
            &ticket_id,
            to,
            &String::from_str(&self.env, "Stellar Concert"),
            &String::from_str(&self.env, "Arena"),
            &2_000_000_000_u64,
            &String::from_str(&self.env, "A1"),
        );
    }
}

#[test]
fn test_secondary_transfer_legal_price_succeeds() {
    let f = Fixture::new();

    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(1_u32, &fan_a);

    // Fast-forward past the transfer cooldown
    f.env.ledger().set_timestamp(70);

    let sale_price = 105_i128;
    f.client().transfer_ticket(
        &1_u32,
        &1_u32,
        &fan_a,
        &fan_b,
        &sale_price,
        &f.token_id,
    );

    let record = f.client().get_ticket(&1_u32, &1_u32).expect("ticket must exist");
    assert_eq!(record.owner, fan_b);
    assert!(!record.is_used);
}

#[test]
#[should_panic]
fn test_scalper_blocked_price_above_ceiling() {
    let f = Fixture::new();

    let fan_b = Address::generate(&f.env);
    let fan_c = Address::generate(&f.env);

    f.fund(&fan_c, 1_000);
    f.mint(2_u32, &fan_b);

    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &2_u32,
        &fan_b,
        &fan_c,
        &200_i128,
        &f.token_id,
    );
}

#[test]
fn test_check_in_marks_ticket_as_used() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);

    f.mint(3_u32, &fan_a);
    f.client().check_in(&1_u32, &3_u32, &fan_a);

    let record = f.client().get_ticket(&1_u32, &3_u32).expect("ticket must exist");
    assert!(record.is_used);
}

#[test]
#[should_panic]
fn test_used_ticket_cannot_be_transferred() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);

    f.mint(4_u32, &fan_a);
    f.client().check_in(&1_u32, &4_u32, &fan_a);

    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &4_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );
}

#[test]
#[should_panic]
fn test_double_initialize_panics() {
    let f = Fixture::new();
    f.client().create_event(
        &1_u32,
        &f.admin,
        &f.token_id,
        &100_i128,
        &110_u32,
        &500_u32,
        &f.royalty_wallet,
        &1_000_u32,
        &4_u32,
        &60_u64,
    );
}

#[test]
fn test_refund_works() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);

    f.fund(&f.admin, 1000);
    f.mint(10_u32, &fan_a);

    let bal_before = f.token().balance(&fan_a);
    f.client().refund_ticket(&1_u32, &10_u32, &f.token_id);
    let bal_after = f.token().balance(&fan_a);

    assert_eq!(bal_after - bal_before, 100_i128);
    let record = f.client().get_ticket(&1_u32, &10_u32).unwrap();
    assert!(record.is_refunded);
    assert_eq!(f.client().get_user_ticket_balance(&1_u32, &fan_a), 0);
}

#[test]
#[should_panic]
fn test_whitelist_blocks_transfer() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(11_u32, &fan_a);

    f.client().set_whitelist_enabled(&1_u32, &true);
    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &11_u32,
        &fan_a,
        &fan_b,
        &105_i128,
        &f.token_id,
    );
}

#[test]
fn test_whitelist_allows_transfer_when_whitelisted() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(12_u32, &fan_a);

    f.client().set_whitelist_enabled(&1_u32, &true);
    f.client().add_to_whitelist(&1_u32, &fan_b);
    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &12_u32,
        &fan_a,
        &fan_b,
        &105_i128,
        &f.token_id,
    );

    let record = f.client().get_ticket(&1_u32, &12_u32).unwrap();
    assert_eq!(record.owner, fan_b);
}

#[test]
fn test_batch_mint_tickets_succeeds() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    let mut batch = Vec::new(&f.env);
    batch.push_back((
        21_u32,
        fan_a.clone(),
        String::from_str(&f.env, "Festival"),
        String::from_str(&f.env, "Park"),
        2_000_000_000_u64,
        String::from_str(&f.env, "VIP-1"),
    ));
    batch.push_back((
        22_u32,
        fan_b.clone(),
        String::from_str(&f.env, "Festival"),
        String::from_str(&f.env, "Park"),
        2_000_000_000_u64,
        String::from_str(&f.env, "VIP-2"),
    ));

    f.client().batch_mint_tickets(&1_u32, &batch);

    let t1 = f.client().get_ticket(&1_u32, &21_u32).unwrap();
    assert_eq!(t1.owner, fan_a);
    let t2 = f.client().get_ticket(&1_u32, &22_u32).unwrap();
    assert_eq!(t2.owner, fan_b);
}

#[test]
#[should_panic]
fn test_expired_ticket_cannot_be_checked_in() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);

    f.client().mint_ticket(
        &1_u32,
        &31_u32,
        &fan_a,
        &String::from_str(&f.env, "Old Event"),
        &String::from_str(&f.env, "Hall"),
        &1_000_u64,
        &String::from_str(&f.env, "B1"),
    );

    f.env.ledger().set_timestamp(1_001);

    f.client().check_in(&1_u32, &31_u32, &fan_a);
}

#[test]
#[should_panic]
fn test_refunded_ticket_cannot_be_transferred() {
    let f     = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&f.admin, 1000);
    f.fund(&fan_b, 500);
    f.mint(41_u32, &fan_a);

    f.client().refund_ticket(&1_u32, &41_u32, &f.token_id);

    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &41_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );
}

#[test]
#[should_panic]
fn test_token_spoofing_rejected() {
    let f = Fixture::new();

    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    // Deploy an unapproved fake token
    let fake_asset = f.env.register_stellar_asset_contract_v2(f.admin.clone());
    let fake_token = fake_asset.address();

    f.mint(51_u32, &fan_a);
    f.env.ledger().set_timestamp(70);

    // Attempt to transfer ticket using the counterfeit token
    f.client().transfer_ticket(
        &1_u32,
        &51_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &fake_token,
    );
}

#[test]
#[should_panic]
fn test_wallet_quota_rate_limiter_blocks_scalper() {
    let f = Fixture::new();
    let scalper = Address::generate(&f.env);

    // Quota is 4 tickets max per wallet
    f.mint(61_u32, &scalper);
    f.mint(62_u32, &scalper);
    f.mint(63_u32, &scalper);
    f.mint(64_u32, &scalper);

    // 5th ticket should be blocked
    f.mint(65_u32, &scalper);
}

#[test]
#[should_panic]
fn test_velocity_cooldown_blocks_rapid_flip() {
    let f = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(71_u32, &fan_a);

    // Cooldown is 60s, attempting transfer at timestamp 30s should panic
    f.env.ledger().set_timestamp(30);

    f.client().transfer_ticket(
        &1_u32,
        &71_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );
}

#[test]
fn test_velocity_cooldown_passes_after_duration() {
    let f = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(81_u32, &fan_a);

    // Pass the 60s cooldown
    f.env.ledger().set_timestamp(61);

    f.client().transfer_ticket(
        &1_u32,
        &81_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );

    let ticket = f.client().get_ticket(&1_u32, &81_u32).unwrap();
    assert_eq!(ticket.owner, fan_b);
}

#[test]
#[should_panic]
fn test_supply_cap_enforced() {
    let f = Fixture::new();
    let admin = Address::generate(&f.env);
    let fan = Address::generate(&f.env);

    // Create event with max_supply = 2
    f.client().create_event(
        &2_u32,
        &admin,
        &f.token_id,
        &100_i128,
        &110_u32,
        &500_u32,
        &f.royalty_wallet,
        &2_u32, // max_supply = 2
        &4_u32,
        &60_u64,
    );

    f.client().mint_ticket(&2_u32, &1_u32, &fan, &String::from_str(&f.env, "T1"), &String::from_str(&f.env, "V"), &2000000000_u64, &String::from_str(&f.env, "S1"));
    f.client().mint_ticket(&2_u32, &2_u32, &fan, &String::from_str(&f.env, "T2"), &String::from_str(&f.env, "V"), &2000000000_u64, &String::from_str(&f.env, "S2"));

    // 3rd ticket exceeds supply cap
    f.client().mint_ticket(&2_u32, &3_u32, &fan, &String::from_str(&f.env, "T3"), &String::from_str(&f.env, "V"), &2000000000_u64, &String::from_str(&f.env, "S3"));
}

#[test]
#[should_panic]
fn test_circuit_breaker_pauses_transfers() {
    let f = Fixture::new();
    let fan_a = Address::generate(&f.env);
    let fan_b = Address::generate(&f.env);

    f.fund(&fan_b, 500);
    f.mint(91_u32, &fan_a);

    // Event admin activates the emergency circuit breaker
    f.client().set_paused(&1_u32, &true);

    f.env.ledger().set_timestamp(70);

    f.client().transfer_ticket(
        &1_u32,
        &91_u32,
        &fan_a,
        &fan_b,
        &100_i128,
        &f.token_id,
    );
}

#[test]
#[should_panic]
fn test_invalid_royalty_rejected_on_creation() {
    let f = Fixture::new();
    let admin = Address::generate(&f.env);

    // Attempting royalty > 5000 bps (50%) should be rejected
    f.client().create_event(
        &3_u32,
        &admin,
        &f.token_id,
        &100_i128,
        &110_u32,
        &6_000_u32, // 60% royalty is invalid
        &f.royalty_wallet,
        &100_u32,
        &4_u32,
        &60_u64,
    );
}