#![no_std]

// Permissioned Testnet oracle adapter. This is NOT an independent price feed
// and cannot be trusted for production borrowing until feed verification,
// decentralised governance, stale/circuit-breaker policies and audits are done.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    Env,
};

const TTL_THRESHOLD: u32 = 518_400;
const TTL_BUMP: u32 = 1_036_800;

#[contracttype]
#[derive(Clone)]
pub enum Key {
    Admin,
    Paused,
    Feeder(Address),
    Quote(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceQuote {
    // Fixed USD scaling of 10^7, regardless of the underlying token decimals.
    pub price: i128,
    pub updated_at: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OracleError {
    Paused = 1,
    UnauthorizedFeeder = 2,
    InvalidPrice = 3,
    MissingQuote = 4,
}

#[contractevent]
pub struct QuoteUpdated {
    pub asset: Address,
    pub price: i128,
    pub timestamp: u64,
}

#[contract]
pub struct HoxoPriceOracle;

fn keep(env: &Env) {
    env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

#[contractimpl]
impl HoxoPriceOracle {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&Key::Admin, &admin);
        env.storage().instance().set(&Key::Paused, &true);
        env.storage().instance().set(&Key::Feeder(admin), &true);
        keep(&env);
    }

    pub fn admin(env: Env) -> Address {
        keep(&env);
        env.storage().instance().get(&Key::Admin).unwrap()
    }

    pub fn paused(env: Env) -> bool {
        keep(&env);
        env.storage().instance().get(&Key::Paused).unwrap_or(true)
    }

    pub fn set_paused(env: Env, paused: bool) {
        Self::admin(env.clone()).require_auth();
        env.storage().instance().set(&Key::Paused, &paused);
        keep(&env);
    }

    pub fn set_feeder(env: Env, feeder: Address, allowed: bool) {
        Self::admin(env.clone()).require_auth();
        env.storage().instance().set(&Key::Feeder(feeder), &allowed);
        keep(&env);
    }

    pub fn is_feeder(env: Env, feeder: Address) -> bool {
        keep(&env);
        env.storage()
            .instance()
            .get(&Key::Feeder(feeder))
            .unwrap_or(false)
    }

    pub fn submit(env: Env, feeder: Address, asset: Address, price: i128) {
        if price <= 0 {
            panic_with_error!(&env, OracleError::InvalidPrice);
        }
        if Self::paused(env.clone()) {
            panic_with_error!(&env, OracleError::Paused);
        }
        feeder.require_auth();
        if !Self::is_feeder(env.clone(), feeder) {
            panic_with_error!(&env, OracleError::UnauthorizedFeeder);
        }
        let updated_at = env.ledger().timestamp();
        let quote = PriceQuote { price, updated_at };
        let key = Key::Quote(asset.clone());
        env.storage().persistent().set(&key, &quote);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        keep(&env);
        QuoteUpdated {
            asset,
            price,
            timestamp: updated_at,
        }
        .publish(&env);
    }

    pub fn read(env: Env, asset: Address) -> PriceQuote {
        keep(&env);
        let key = Key::Quote(asset);
        let quote = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::MissingQuote));
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        quote
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn starts_paused_and_records_authenticated_prices() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|info| info.timestamp = 10_000);
        let admin = Address::generate(&env);
        let feed = Address::generate(&env);
        let asset = Address::generate(&env);
        let id = env.register(HoxoPriceOracle, (&admin,));
        let oracle = HoxoPriceOracleClient::new(&env, &id);
        assert!(oracle.paused());
        oracle.set_feeder(&feed, &true);
        oracle.set_paused(&false);
        oracle.submit(&feed, &asset, &22_300_000);
        assert_eq!(
            oracle.read(&asset),
            PriceQuote { price: 22_300_000, updated_at: 10_000 }
        );
        assert!(oracle.is_feeder(&feed));
    }

    #[test]
    #[should_panic]
    fn rejects_non_feeder_updates() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let other = Address::generate(&env);
        let asset = Address::generate(&env);
        let id = env.register(HoxoPriceOracle, (&admin,));
        let oracle = HoxoPriceOracleClient::new(&env, &id);
        oracle.set_paused(&false);
        oracle.submit(&other, &asset, &100_000_000);
    }

    #[test]
    #[should_panic]
    fn rejects_zero_prices() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let id = env.register(HoxoPriceOracle, (&admin,));
        let oracle = HoxoPriceOracleClient::new(&env, &id);
        oracle.set_paused(&false);
        oracle.submit(&admin, &asset, &0);
    }
}
