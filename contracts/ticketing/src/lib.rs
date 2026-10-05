#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, panic_with_error,
    token::Client as TokenClient,
    Address, Env, String, Vec, symbol_short,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Event(u32),
    Ticket(u32, u32),
    Whitelist(u32, Address),
    UserTicketCount(u32, Address),
}

#[contracttype]
#[derive(Clone)]
pub struct EventConfig {
    pub admin: Address,
    pub payment_token: Address,
    pub face_value: i128,
    pub max_resale_multiplier: u32,
    pub royalty_basis_points: u32,
    pub royalty_recipient: Address,
    pub whitelist_enabled: bool,
    pub max_supply: u32,
    pub current_supply: u32,
    pub max_tickets_per_wallet: u32,
    pub transfer_cooldown_seconds: u64,
    pub is_paused: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct TicketRecord {
    pub owner: Address,
    pub is_used: bool,
    pub is_refunded: bool,
    pub title: String,
    pub venue: String,
    pub date: u64,
    pub seat: String,
    pub last_transfer_timestamp: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    EventAlreadyExists          = 1,
    EventNotFound               = 2,
    TicketAlreadyExists         = 3,
    TicketNotFound              = 4,
    TicketAlreadyUsed           = 5,
    NotTicketOwner              = 6,
    PriceCeilingExceeded        = 7,
    ArithmeticOverflow          = 8,
    TicketExpired               = 9,
    NotAuthorized               = 10,
    NotWhitelisted              = 11,
    TicketRefunded              = 12,
    InvalidParameter            = 13,
    InvalidPaymentToken         = 14,
    SupplyExceeded              = 15,
    WalletHoldingLimitExceeded  = 16,
    TransferCooldownActive      = 17,
    EventPaused                 = 18,
}

const INSTANCE_TTL_LOW:  u32 = 17_280;
const INSTANCE_TTL_HIGH: u32 = 17_280;
const TICKET_TTL_LOW:    u32 = 518_400;
const TICKET_TTL_HIGH:   u32 = 518_400;

#[contract]
pub struct StellarPassContract;

#[contractimpl]
impl StellarPassContract {
    /// Creates a new ticketed event with security boundaries and rate limits.
    pub fn create_event(
        env:                       Env,
        event_id:                  u32,
        admin:                     Address,
        payment_token:             Address,
        face_value:                i128,
        max_resale_multiplier:     u32,
        royalty_basis_points:      u32,
        royalty_recipient:         Address,
        max_supply:                u32,
        max_tickets_per_wallet:    u32,
        transfer_cooldown_seconds: u64,
    ) {
        admin.require_auth();

        if face_value <= 0
            || max_resale_multiplier < 100
            || royalty_basis_points > 5_000
            || max_supply == 0
            || max_tickets_per_wallet == 0
        {
            panic_with_error!(&env, Error::InvalidParameter);
        }

        let key = DataKey::Event(event_id);
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, Error::EventAlreadyExists);
        }

        let config = EventConfig {
            admin,
            payment_token,
            face_value,
            max_resale_multiplier,
            royalty_basis_points,
            royalty_recipient,
            whitelist_enabled: false,
            max_supply,
            current_supply: 0,
            max_tickets_per_wallet,
            transfer_cooldown_seconds,
            is_paused: false,
        };

        env.storage().persistent().set(&key, &config);
        env.storage().persistent().extend_ttl(&key, INSTANCE_TTL_LOW, INSTANCE_TTL_HIGH);
    }

    /// Emergency pause switch to halt all activity in case of attacks or venue cancellation.
    pub fn set_paused(env: Env, event_id: u32, paused: bool) {
        let mut config = Self::require_event(&env, event_id);
        config.admin.require_auth();
        config.is_paused = paused;
        let key = DataKey::Event(event_id);
        env.storage().persistent().set(&key, &config);
    }

    /// Transfers the event administrator authority to a new address.
    pub fn set_event_admin(env: Env, event_id: u32, new_admin: Address) {
        let mut config = Self::require_event(&env, event_id);
        config.admin.require_auth();
        config.admin = new_admin;
        let key = DataKey::Event(event_id);
        env.storage().persistent().set(&key, &config);
    }

    pub fn set_whitelist_enabled(env: Env, event_id: u32, enabled: bool) {
        let mut config = Self::require_event(&env, event_id);
        config.admin.require_auth();
        config.whitelist_enabled = enabled;
        let key = DataKey::Event(event_id);
        env.storage().persistent().set(&key, &config);
    }

    pub fn add_to_whitelist(env: Env, event_id: u32, user: Address) {
        let config = Self::require_event(&env, event_id);
        config.admin.require_auth();
        let key = DataKey::Whitelist(event_id, user);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
    }

    pub fn mint_ticket(
        env: Env,
        event_id: u32,
        ticket_id: u32,
        to: Address,
        title: String,
        venue: String,
        date: u64,
        seat: String,
    ) {
        let mut config = Self::require_event(&env, event_id);
        config.admin.require_auth();

        if config.is_paused {
            panic_with_error!(&env, Error::EventPaused);
        }

        if config.current_supply >= config.max_supply {
            panic_with_error!(&env, Error::SupplyExceeded);
        }

        let key = DataKey::Ticket(event_id, ticket_id);
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, Error::TicketAlreadyExists);
        }

        Self::increment_user_ticket_count(&env, event_id, &to, config.max_tickets_per_wallet);

        config.current_supply += 1;
        env.storage().persistent().set(&DataKey::Event(event_id), &config);

        let now = env.ledger().timestamp();
        env.storage().persistent().set(
            &key,
            &TicketRecord {
                owner: to.clone(),
                is_used: false,
                is_refunded: false,
                title,
                venue,
                date,
                seat,
                last_transfer_timestamp: now,
            },
        );
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);

        env.events().publish((symbol_short!("mint"), event_id, ticket_id), to);
    }

    pub fn batch_mint_tickets(
        env: Env,
        event_id: u32,
        tickets: Vec<(u32, Address, String, String, u64, String)>,
    ) {
        let mut config = Self::require_event(&env, event_id);
        config.admin.require_auth();

        if config.is_paused {
            panic_with_error!(&env, Error::EventPaused);
        }

        let batch_len = tickets.len();
        if config.current_supply.checked_add(batch_len).unwrap_or(u32::MAX) > config.max_supply {
            panic_with_error!(&env, Error::SupplyExceeded);
        }

        let now = env.ledger().timestamp();

        for ticket in tickets.into_iter() {
            let (ticket_id, to, title, venue, date, seat) = ticket;

            let key = DataKey::Ticket(event_id, ticket_id);
            if env.storage().persistent().has(&key) {
                panic_with_error!(&env, Error::TicketAlreadyExists);
            }

            Self::increment_user_ticket_count(&env, event_id, &to, config.max_tickets_per_wallet);

            env.storage().persistent().set(
                &key,
                &TicketRecord {
                    owner: to.clone(),
                    is_used: false,
                    is_refunded: false,
                    title,
                    venue,
                    date,
                    seat,
                    last_transfer_timestamp: now,
                },
            );
            env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);

            env.events().publish((symbol_short!("mint"), event_id, ticket_id), to);
        }

        config.current_supply += batch_len;
        env.storage().persistent().set(&DataKey::Event(event_id), &config);
    }

    pub fn transfer_ticket(
        env:        Env,
        event_id:   u32,
        ticket_id:  u32,
        from:       Address,
        to:         Address,
        amount:     i128,
        token_addr: Address,
    ) {
        from.require_auth();
        to.require_auth();

        let config = Self::require_event(&env, event_id);

        if config.is_paused {
            panic_with_error!(&env, Error::EventPaused);
        }

        if token_addr != config.payment_token {
            panic_with_error!(&env, Error::InvalidPaymentToken);
        }

        if amount < 0 {
            panic_with_error!(&env, Error::InvalidParameter);
        }

        let key = DataKey::Ticket(event_id, ticket_id);
        let mut record: TicketRecord = Self::require_ticket(&env, event_id, ticket_id);

        if record.owner != from {
            panic_with_error!(&env, Error::NotTicketOwner);
        }
        if record.is_used {
            panic_with_error!(&env, Error::TicketAlreadyUsed);
        }
        if record.is_refunded {
            panic_with_error!(&env, Error::TicketRefunded);
        }
        if env.ledger().timestamp() > record.date {
            panic_with_error!(&env, Error::TicketExpired);
        }

        let now = env.ledger().timestamp();
        if now < record.last_transfer_timestamp.saturating_add(config.transfer_cooldown_seconds) {
            panic_with_error!(&env, Error::TransferCooldownActive);
        }

        if config.whitelist_enabled {
            let wl_key = DataKey::Whitelist(event_id, to.clone());
            if !env.storage().persistent().has(&wl_key) {
                panic_with_error!(&env, Error::NotWhitelisted);
            }
        }

        Self::increment_user_ticket_count(&env, event_id, &to, config.max_tickets_per_wallet);
        Self::decrement_user_ticket_count(&env, event_id, &from);

        let max_allowed: i128 = config.face_value
            .checked_mul(config.max_resale_multiplier as i128)
            .and_then(|v| v.checked_div(100))
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

        if amount > max_allowed {
            panic_with_error!(&env, Error::PriceCeilingExceeded);
        }

        if amount > 0 {
            let token = TokenClient::new(&env, &token_addr);
            let this  = env.current_contract_address();

            if from == config.admin {
                token.transfer(&to, &config.admin, &amount);
            } else {
                let royalty: i128 = amount
                    .checked_mul(config.royalty_basis_points as i128)
                    .and_then(|v| v.checked_div(10_000))
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

                let seller_proceeds: i128 = amount
                    .checked_sub(royalty)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));

                token.transfer(&to, &this, &amount);
                if royalty > 0 {
                    token.transfer(&this, &config.royalty_recipient, &royalty);
                }
                if seller_proceeds > 0 {
                    token.transfer(&this, &from, &seller_proceeds);
                }
            }
        }

        record.owner = to.clone();
        record.last_transfer_timestamp = now;
        env.storage().persistent().set(&key, &record);
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);

        env.events().publish((symbol_short!("transfer"), event_id, ticket_id), (from, to, amount));
    }

    pub fn check_in(env: Env, event_id: u32, ticket_id: u32, owner: Address) {
        owner.require_auth();

        let config = Self::require_event(&env, event_id);
        if config.is_paused {
            panic_with_error!(&env, Error::EventPaused);
        }

        let key        = DataKey::Ticket(event_id, ticket_id);
        let mut record = Self::require_ticket(&env, event_id, ticket_id);

        if record.owner != owner {
            panic_with_error!(&env, Error::NotTicketOwner);
        }
        if record.is_used {
            panic_with_error!(&env, Error::TicketAlreadyUsed);
        }
        if record.is_refunded {
            panic_with_error!(&env, Error::TicketRefunded);
        }
        if env.ledger().timestamp() > record.date {
            panic_with_error!(&env, Error::TicketExpired);
        }

        record.is_used = true;
        env.storage().persistent().set(&key, &record);
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);

        env.events().publish((symbol_short!("check_in"), event_id, ticket_id), owner);
    }

    pub fn refund_ticket(env: Env, event_id: u32, ticket_id: u32, token_addr: Address) {
        let config = Self::require_event(&env, event_id);
        config.admin.require_auth();

        if token_addr != config.payment_token {
            panic_with_error!(&env, Error::InvalidPaymentToken);
        }

        let key = DataKey::Ticket(event_id, ticket_id);
        let mut record = Self::require_ticket(&env, event_id, ticket_id);

        if record.is_refunded {
            panic_with_error!(&env, Error::TicketRefunded);
        }
        if record.is_used {
            panic_with_error!(&env, Error::TicketAlreadyUsed);
        }
        if env.ledger().timestamp() > record.date {
            panic_with_error!(&env, Error::TicketExpired);
        }

        record.is_refunded = true;
        env.storage().persistent().set(&key, &record);
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);

        Self::decrement_user_ticket_count(&env, event_id, &record.owner);

        let token = TokenClient::new(&env, &token_addr);
        token.transfer(&config.admin, &record.owner, &config.face_value);

        env.events().publish((symbol_short!("refund"), event_id, ticket_id), record.owner);
    }

    pub fn get_ticket(env: Env, event_id: u32, ticket_id: u32) -> Option<TicketRecord> {
        env.storage().persistent().get(&DataKey::Ticket(event_id, ticket_id))
    }

    pub fn get_event(env: Env, event_id: u32) -> Option<EventConfig> {
        env.storage().persistent().get(&DataKey::Event(event_id))
    }

    pub fn get_user_ticket_balance(env: Env, event_id: u32, user: Address) -> u32 {
        Self::get_user_ticket_count(&env, event_id, &user)
    }

    fn require_event(env: &Env, event_id: u32) -> EventConfig {
        env.storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::EventNotFound))
    }

    fn require_ticket(env: &Env, event_id: u32, ticket_id: u32) -> TicketRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Ticket(event_id, ticket_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::TicketNotFound))
    }

    fn get_user_ticket_count(env: &Env, event_id: u32, user: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::UserTicketCount(event_id, user.clone()))
            .unwrap_or(0)
    }

    fn increment_user_ticket_count(env: &Env, event_id: u32, user: &Address, max_allowed: u32) {
        let current = Self::get_user_ticket_count(env, event_id, user);
        if current >= max_allowed {
            panic_with_error!(env, Error::WalletHoldingLimitExceeded);
        }
        let key = DataKey::UserTicketCount(event_id, user.clone());
        env.storage().persistent().set(&key, &(current + 1));
        env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
    }

    fn decrement_user_ticket_count(env: &Env, event_id: u32, user: &Address) {
        let current = Self::get_user_ticket_count(env, event_id, user);
        if current > 0 {
            let key = DataKey::UserTicketCount(event_id, user.clone());
            env.storage().persistent().set(&key, &(current - 1));
            env.storage().persistent().extend_ttl(&key, TICKET_TTL_LOW, TICKET_TTL_HIGH);
        }
    }
}

mod test;