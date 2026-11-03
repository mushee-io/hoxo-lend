#![no_std]

// The Hoxo Lend Soroban Lens is a read-only composition adapter. Nothing
// here mutates market positions, signs a wallet, or fabricates APR/TVL.

use soroban_sdk::{contract, contractclient, contractimpl, contracttype, Address, Env};

const INDEX: i128 = 1_000_000_000_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskConfig {
    pub ltv_bps: u32,
    pub liquidation_bps: u32,
    pub liquidation_bonus_bps: u32,
    pub reserve_factor_bps: u32,
    pub supply_cap: i128,
    pub borrow_cap: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketState {
    pub supply_index: i128,
    pub borrow_index: i128,
    pub supply_shares: i128,
    pub debt_shares: i128,
    pub reserves: i128,
    pub last_accrual: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub supply_shares: i128,
    pub debt_shares: i128,
    pub collateral: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskView {
    pub collateral_usd: i128,
    pub debt_usd: i128,
    pub borrow_limit_usd: i128,
    pub liquidation_limit_usd: i128,
    pub healthy: bool,
    pub inside_ltv: bool,
}

#[contractclient(name = "MarketClient")]
pub trait MarketInterface {
    fn config(env: Env) -> RiskConfig;
    fn market_state(env: Env) -> MarketState;
    fn available_liquidity(env: Env) -> i128;
    fn position_of(env: Env, user: Address) -> Position;
    fn supplied_of(env: Env, user: Address) -> i128;
    fn borrowed_of(env: Env, user: Address) -> i128;
    fn risk_of(env: Env, user: Address) -> RiskView;
    fn paused(env: Env) -> bool;
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketSnapshot {
    pub total_supply_claims: i128,
    pub total_debt: i128,
    pub cash: i128,
    pub reserves: i128,
    pub utilization_bps: u32,
    pub supply_index: i128,
    pub borrow_index: i128,
    pub last_accrual: u64,
    pub supply_cap: i128,
    pub borrow_cap: i128,
    pub paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSnapshot {
    pub supplied: i128,
    pub borrowed: i128,
    pub collateral: i128,
    pub health: RiskView,
}

#[contract]
pub struct HoxoLendLens;

#[contractimpl]
impl HoxoLendLens {
    pub fn market(env: Env, market: Address) -> MarketSnapshot {
        let client = MarketClient::new(&env, &market);
        let s = client.market_state();
        let c = client.config();
        let supplied = s.supply_shares * s.supply_index / INDEX;
        let borrowed = s.debt_shares * s.borrow_index / INDEX;
        let util = if supplied == 0 {
            0
        } else if borrowed >= supplied {
            10_000
        } else {
            (borrowed * 10_000 / supplied) as u32
        };
        MarketSnapshot {
            total_supply_claims: supplied,
            total_debt: borrowed,
            cash: client.available_liquidity(),
            reserves: s.reserves,
            utilization_bps: util,
            supply_index: s.supply_index,
            borrow_index: s.borrow_index,
            last_accrual: s.last_accrual,
            supply_cap: c.supply_cap,
            borrow_cap: c.borrow_cap,
            paused: client.paused(),
        }
    }

    pub fn account(env: Env, market: Address, user: Address) -> AccountSnapshot {
        let client = MarketClient::new(&env, &market);
        let pos = client.position_of(&user);
        AccountSnapshot {
            supplied: client.supplied_of(&user),
            borrowed: client.borrowed_of(&user),
            collateral: pos.collateral,
            health: client.risk_of(&user),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[contract]
    struct MockMarket;

    #[contractimpl]
    impl MockMarket {
        pub fn config(_env: Env) -> RiskConfig {
            RiskConfig {
                ltv_bps: 6000,
                liquidation_bps: 7500,
                liquidation_bonus_bps: 500,
                reserve_factor_bps: 1000,
                supply_cap: 1_000_000,
                borrow_cap: 500_000,
            }
        }
        pub fn market_state(_env: Env) -> MarketState {
            MarketState {
                supply_index: INDEX,
                borrow_index: INDEX,
                supply_shares: 100_000,
                debt_shares: 40_000,
                reserves: 100,
                last_accrual: 10,
            }
        }
        pub fn available_liquidity(_env: Env) -> i128 {
            60_000
        }
        pub fn position_of(_env: Env, _user: Address) -> Position {
            Position {
                supply_shares: 0,
                debt_shares: 20_000,
                collateral: 70_000,
            }
        }
        pub fn supplied_of(_env: Env, _user: Address) -> i128 {
            0
        }
        pub fn borrowed_of(_env: Env, _user: Address) -> i128 {
            20_000
        }
        pub fn risk_of(_env: Env, _user: Address) -> RiskView {
            RiskView {
                collateral_usd: 30_000,
                debt_usd: 20_000,
                borrow_limit_usd: 18_000,
                liquidation_limit_usd: 22_500,
                healthy: true,
                inside_ltv: false,
            }
        }
        pub fn paused(_env: Env) -> bool {
            true
        }
    }

    #[test]
    fn reads_real_market_interface_without_write_methods() {
        let env = Env::default();
        let market = env.register(MockMarket, ());
        let lens = env.register(HoxoLendLens, ());
        let viewer = HoxoLendLensClient::new(&env, &lens);
        let stats = viewer.market(&market);
        assert_eq!(stats.total_supply_claims, 100_000);
        assert_eq!(stats.total_debt, 40_000);
        assert_eq!(stats.cash, 60_000);
        assert!(stats.paused);
    }
}
