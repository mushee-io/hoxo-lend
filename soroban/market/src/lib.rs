#![no_std]

// Hoxo Lend: one isolated collateral/debt pair on Stellar Soroban.
// This prototype is UNDEPLOYED AND UNAUDITED. It is not safe for real funds.
// Price feeds are external, permissioned Testnet-only contracts.
// No production credit should be enabled until formal economic review.

use soroban_sdk::{
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    panic_with_error, token, Address, Env,
};

const BPS: i128 = 10_000;
const INDEX: i128 = 1_000_000_000_000;
const YEAR_SECONDS: i128 = 31_536_000;
const MAX_ORACLE_AGE: u64 = 1_800;
const TTL_THRESHOLD: u32 = 518_400;
const TTL_BUMP: u32 = 1_036_800;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceQuote {
    pub price: i128, // USD price scaled to 10^7
    pub updated_at: u64,
}

#[contractclient(name = "OracleClient")]
pub trait OracleInterface {
    fn read(env: Env, asset: Address) -> PriceQuote;
}

#[contractclient(name = "RateModelClient")]
pub trait RateModelInterface {
    fn borrow_rate_bps(env: Env, utilisation_bps: u32) -> u32;
}

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

#[contracttype]
#[derive(Clone)]
enum Key {
    Admin,
    Guardian,
    LoanAsset,
    CollateralAsset,
    Oracle,
    RateModel,
    Config,
    Market,
    Paused,
    Entered,
    Position(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MarketError {
    InvalidAmount = 1,
    InvalidConfig = 2,
    Paused = 3,
    InsufficientBalance = 4,
    InsufficientLiquidity = 5,
    LimitExceeded = 6,
    OracleStale = 7,
    OracleInvalid = 8,
    Undercollateralized = 9,
    NotLiquidatable = 10,
    Overflow = 11,
    Reentrancy = 12,
    Dust = 13,
    BadDebtNotEligible = 14,
}

#[contractevent]
pub struct MarketAction {
    pub user: Address,
    pub action: u32,
    pub amount: i128,
}

#[contract]
pub struct HoxoLendingMarket;

fn keep(env: &Env) {
    env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

fn checked(env: &Env, n: Option<i128>) -> i128 {
    n.unwrap_or_else(|| panic_with_error!(env, MarketError::Overflow))
}

fn multiply_div(env: &Env, a: i128, b: i128, denominator: i128) -> i128 {
    if denominator <= 0 || a < 0 || b < 0 {
        panic_with_error!(env, MarketError::InvalidConfig);
    }
    checked(env, a.checked_mul(b)) / denominator
}

fn ceil_div(env: &Env, x: i128, y: i128) -> i128 {
    if x < 0 || y <= 0 {
        panic_with_error!(env, MarketError::InvalidConfig);
    }
    if x == 0 {
        0
    } else {
        checked(env, x.checked_sub(1)) / y + 1
    }
}

fn cfg(env: &Env) -> RiskConfig {
    env.storage().instance().get(&Key::Config).unwrap()
}

fn state(env: &Env) -> MarketState {
    env.storage().instance().get(&Key::Market).unwrap()
}

fn save_state(env: &Env, s: &MarketState) {
    env.storage().instance().set(&Key::Market, s);
    keep(env);
}

fn admin(env: &Env) -> Address {
    env.storage().instance().get(&Key::Admin).unwrap()
}

fn loan_asset(env: &Env) -> Address {
    env.storage().instance().get(&Key::LoanAsset).unwrap()
}

fn collateral_asset(env: &Env) -> Address {
    env.storage().instance().get(&Key::CollateralAsset).unwrap()
}

fn position(env: &Env, user: &Address) -> Position {
    let key = Key::Position(user.clone());
    let result = env.storage().persistent().get(&key).unwrap_or(Position {
        supply_shares: 0,
        debt_shares: 0,
        collateral: 0,
    });
    if env.storage().persistent().has(&key) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    }
    result
}

fn store_position(env: &Env, user: &Address, p: &Position) {
    let key = Key::Position(user.clone());
    env.storage().persistent().set(&key, p);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

fn supply_total(env: &Env, s: &MarketState) -> i128 {
    multiply_div(env, s.supply_shares, s.supply_index, INDEX)
}

fn debt_total(env: &Env, s: &MarketState) -> i128 {
    multiply_div(env, s.debt_shares, s.borrow_index, INDEX)
}

fn supplied(env: &Env, p: &Position, s: &MarketState) -> i128 {
    multiply_div(env, p.supply_shares, s.supply_index, INDEX)
}

fn borrowed(env: &Env, p: &Position, s: &MarketState) -> i128 {
    ceil_div(
        env,
        checked(env, p.debt_shares.checked_mul(s.borrow_index)),
        INDEX,
    )
}

fn debt_amount_total(env: &Env, s: &MarketState) -> i128 {
    ceil_div(
        env,
        checked(env, s.debt_shares.checked_mul(s.borrow_index)),
        INDEX,
    )
}

fn cash(env: &Env) -> i128 {
    token::Client::new(env, &loan_asset(env)).balance(&env.current_contract_address())
}

fn enter(env: &Env) {
    let occupied: bool = env.storage().instance().get(&Key::Entered).unwrap_or(false);
    if occupied {
        panic_with_error!(env, MarketError::Reentrancy);
    }
    env.storage().instance().set(&Key::Entered, &true);
}

fn leave(env: &Env) {
    env.storage().instance().set(&Key::Entered, &false);
    keep(env);
}

fn not_paused(env: &Env) {
    let paused: bool = env.storage().instance().get(&Key::Paused).unwrap_or(true);
    if paused {
        panic_with_error!(env, MarketError::Paused);
    }
}

fn validate_config(env: &Env, c: &RiskConfig) {
    if c.ltv_bps > 9_000
        || c.ltv_bps >= c.liquidation_bps
        || c.liquidation_bps > 9_500
        || c.liquidation_bonus_bps > 2_000
        || c.reserve_factor_bps > 3_000
        || c.supply_cap <= 0
        || c.borrow_cap <= 0
        || c.borrow_cap > c.supply_cap
    {
        panic_with_error!(env, MarketError::InvalidConfig);
    }
}

fn units(env: &Env, asset: &Address) -> i128 {
    let decimals = token::Client::new(env, asset).decimals();
    if decimals > 18 {
        panic_with_error!(env, MarketError::InvalidConfig);
    }
    let mut result = 1_i128;
    for _ in 0..decimals {
        result = checked(env, result.checked_mul(10));
    }
    result
}

fn price(env: &Env, asset: &Address) -> i128 {
    let oracle: Address = env.storage().instance().get(&Key::Oracle).unwrap();
    let q = OracleClient::new(env, &oracle).read(asset);
    let now = env.ledger().timestamp();
    if q.price <= 0 || q.updated_at > now {
        panic_with_error!(env, MarketError::OracleInvalid);
    }
    if now - q.updated_at > MAX_ORACLE_AGE {
        panic_with_error!(env, MarketError::OracleStale);
    }
    q.price
}

fn risk(env: &Env, p: &Position, s: &MarketState) -> RiskView {
    // Fully debt-free accounts may withdraw without relying on oracle uptime.
    let debt = borrowed(env, p, s);
    if debt == 0 {
        return RiskView {
            collateral_usd: 0,
            debt_usd: 0,
            borrow_limit_usd: 0,
            liquidation_limit_usd: 0,
            healthy: true,
            inside_ltv: true,
        };
    }
    let c = cfg(env);
    let col = collateral_asset(env);
    let debt_token = loan_asset(env);
    let collateral_usd = multiply_div(env, p.collateral, price(env, &col), units(env, &col));
    let debt_usd = ceil_div(
        env,
        checked(env, debt.checked_mul(price(env, &debt_token))),
        units(env, &debt_token),
    );
    let borrow_limit_usd = multiply_div(env, collateral_usd, i128::from(c.ltv_bps), BPS);
    let liquidation_limit_usd =
        multiply_div(env, collateral_usd, i128::from(c.liquidation_bps), BPS);
    RiskView {
        collateral_usd,
        debt_usd,
        borrow_limit_usd,
        liquidation_limit_usd,
        healthy: debt_usd <= liquidation_limit_usd,
        inside_ltv: debt_usd <= borrow_limit_usd,
    }
}

fn accrue(env: &Env) {
    let mut s = state(env);
    let now = env.ledger().timestamp();
    if now <= s.last_accrual {
        return;
    }
    let before_debt = debt_total(env, &s);
    let before_supply = supply_total(env, &s);
    if before_debt == 0 || before_supply == 0 {
        s.last_accrual = now;
        save_state(env, &s);
        return;
    }
    let util_bps = if before_debt >= before_supply {
        10_000
    } else {
        multiply_div(env, before_debt, BPS, before_supply) as u32
    };
    let rate_model: Address = env.storage().instance().get(&Key::RateModel).unwrap();
    let rate_bps = RateModelClient::new(env, &rate_model).borrow_rate_bps(&util_bps);
    if rate_bps > 50_000 {
        panic_with_error!(env, MarketError::InvalidConfig);
    }
    let elapsed = i128::from(now - s.last_accrual);
    let annual = checked(env, i128::from(rate_bps).checked_mul(elapsed));
    let index_delta = multiply_div(
        env,
        s.borrow_index,
        annual,
        checked(env, BPS.checked_mul(YEAR_SECONDS)),
    );
    let new_index = checked(env, s.borrow_index.checked_add(index_delta));
    let after_debt = multiply_div(env, s.debt_shares, new_index, INDEX);
    let interest = checked(env, after_debt.checked_sub(before_debt));
    let reserve_interest =
        multiply_div(env, interest, i128::from(cfg(env).reserve_factor_bps), BPS);
    let supplier_interest = interest - reserve_interest;
    if s.supply_shares > 0 && supplier_interest > 0 {
        let supplier_index_delta = multiply_div(env, supplier_interest, INDEX, s.supply_shares);
        s.supply_index = checked(env, s.supply_index.checked_add(supplier_index_delta));
    }
    s.borrow_index = new_index;
    s.reserves = checked(env, s.reserves.checked_add(reserve_interest));
    s.last_accrual = now;
    save_state(env, &s);
}

#[contractimpl]
impl HoxoLendingMarket {
    pub fn __constructor(
        env: Env,
        admin: Address,
        guardian: Address,
        oracle: Address,
        rate_model: Address,
        collateral_token: Address,
        loan_token: Address,
        config: RiskConfig,
    ) {
        validate_config(&env, &config);
        if collateral_token == loan_token {
            panic_with_error!(&env, MarketError::InvalidConfig);
        }
        // Discover SAC decimals from real token contracts, not ERC-20 metadata.
        units(&env, &collateral_token);
        units(&env, &loan_token);
        env.storage().instance().set(&Key::Admin, &admin);
        env.storage().instance().set(&Key::Guardian, &guardian);
        env.storage().instance().set(&Key::Oracle, &oracle);
        env.storage().instance().set(&Key::RateModel, &rate_model);
        env.storage()
            .instance()
            .set(&Key::CollateralAsset, &collateral_token);
        env.storage().instance().set(&Key::LoanAsset, &loan_token);
        env.storage().instance().set(&Key::Config, &config);
        env.storage().instance().set(&Key::Paused, &true);
        env.storage().instance().set(&Key::Entered, &false);
        env.storage().instance().set(
            &Key::Market,
            &MarketState {
                supply_index: INDEX,
                borrow_index: INDEX,
                supply_shares: 0,
                debt_shares: 0,
                reserves: 0,
                last_accrual: env.ledger().timestamp(),
            },
        );
        keep(&env);
    }

    pub fn paused(env: Env) -> bool {
        keep(&env);
        env.storage().instance().get(&Key::Paused).unwrap_or(true)
    }

    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        let a = admin(&env);
        let guardian: Address = env.storage().instance().get(&Key::Guardian).unwrap();
        if paused {
            if caller != a && caller != guardian {
                panic_with_error!(&env, MarketError::InvalidConfig);
            }
        } else if caller != a {
            panic_with_error!(&env, MarketError::InvalidConfig);
        }
        caller.require_auth();
        env.storage().instance().set(&Key::Paused, &paused);
        keep(&env);
    }

    pub fn configure(env: Env, config: RiskConfig) {
        admin(&env).require_auth();
        enter(&env);
        accrue(&env);
        validate_config(&env, &config);
        // An admin cannot lower a cap below existing aggregate exposure.
        let s = state(&env);
        if supply_total(&env, &s) > config.supply_cap
            || debt_amount_total(&env, &s) > config.borrow_cap
        {
            panic_with_error!(&env, MarketError::LimitExceeded);
        }
        env.storage().instance().set(&Key::Config, &config);
        leave(&env);
    }

    pub fn market_state(env: Env) -> MarketState {
        keep(&env);
        state(&env)
    }

    pub fn config(env: Env) -> RiskConfig {
        keep(&env);
        cfg(&env)
    }

    pub fn position_of(env: Env, user: Address) -> Position {
        keep(&env);
        position(&env, &user)
    }

    pub fn supplied_of(env: Env, user: Address) -> i128 {
        keep(&env);
        supplied(&env, &position(&env, &user), &state(&env))
    }

    pub fn borrowed_of(env: Env, user: Address) -> i128 {
        keep(&env);
        borrowed(&env, &position(&env, &user), &state(&env))
    }

    pub fn risk_of(env: Env, user: Address) -> RiskView {
        keep(&env);
        risk(&env, &position(&env, &user), &state(&env))
    }

    pub fn available_liquidity(env: Env) -> i128 {
        keep(&env);
        cash(&env)
    }

    pub fn accrue_interest(env: Env) {
        enter(&env);
        accrue(&env);
        leave(&env);
    }

    pub fn supply(env: Env, user: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        user.require_auth();
        not_paused(&env);
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let current_total = supply_total(&env, &s);
        if checked(&env, current_total.checked_add(amount)) > cfg(&env).supply_cap {
            panic_with_error!(&env, MarketError::LimitExceeded);
        }
        let shares = multiply_div(&env, amount, INDEX, s.supply_index);
        if shares == 0 {
            panic_with_error!(&env, MarketError::Dust);
        }
        let mut p = position(&env, &user);
        p.supply_shares = checked(&env, p.supply_shares.checked_add(shares));
        s.supply_shares = checked(&env, s.supply_shares.checked_add(shares));
        token::Client::new(&env, &loan_asset(&env)).transfer(
            &user,
            &env.current_contract_address(),
            &amount,
        );
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 1,
            amount,
        }
        .publish(&env);
        leave(&env);
    }

    pub fn withdraw_supply(env: Env, user: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        user.require_auth();
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let mut p = position(&env, &user);
        if supplied(&env, &p, &s) < amount {
            panic_with_error!(&env, MarketError::InsufficientBalance);
        }
        if cash(&env) < amount {
            panic_with_error!(&env, MarketError::InsufficientLiquidity);
        }
        let shares = ceil_div(
            &env,
            checked(&env, amount.checked_mul(INDEX)),
            s.supply_index,
        );
        if shares > p.supply_shares {
            panic_with_error!(&env, MarketError::InsufficientBalance);
        }
        p.supply_shares -= shares;
        s.supply_shares -= shares;
        token::Client::new(&env, &loan_asset(&env)).transfer(
            &env.current_contract_address(),
            &user,
            &amount,
        );
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 2,
            amount,
        }
        .publish(&env);
        leave(&env);
    }

    pub fn deposit_collateral(env: Env, user: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        user.require_auth();
        not_paused(&env);
        enter(&env);
        let mut p = position(&env, &user);
        p.collateral = checked(&env, p.collateral.checked_add(amount));
        token::Client::new(&env, &collateral_asset(&env)).transfer(
            &user,
            &env.current_contract_address(),
            &amount,
        );
        store_position(&env, &user, &p);
        MarketAction {
            user,
            action: 3,
            amount,
        }
        .publish(&env);
        leave(&env);
    }

    pub fn withdraw_collateral(env: Env, user: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        user.require_auth();
        enter(&env);
        accrue(&env);
        let mut p = position(&env, &user);
        if p.collateral < amount {
            panic_with_error!(&env, MarketError::InsufficientBalance);
        }
        p.collateral -= amount;
        if !risk(&env, &p, &state(&env)).healthy {
            panic_with_error!(&env, MarketError::Undercollateralized);
        }
        token::Client::new(&env, &collateral_asset(&env)).transfer(
            &env.current_contract_address(),
            &user,
            &amount,
        );
        store_position(&env, &user, &p);
        MarketAction {
            user,
            action: 4,
            amount,
        }
        .publish(&env);
        leave(&env);
    }

    pub fn borrow(env: Env, user: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        user.require_auth();
        not_paused(&env);
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let mut p = position(&env, &user);
        if cash(&env) < amount {
            panic_with_error!(&env, MarketError::InsufficientLiquidity);
        }
        if checked(&env, debt_amount_total(&env, &s).checked_add(amount)) > cfg(&env).borrow_cap {
            panic_with_error!(&env, MarketError::LimitExceeded);
        }
        let shares = ceil_div(
            &env,
            checked(&env, amount.checked_mul(INDEX)),
            s.borrow_index,
        );
        p.debt_shares = checked(&env, p.debt_shares.checked_add(shares));
        s.debt_shares = checked(&env, s.debt_shares.checked_add(shares));
        if !risk(&env, &p, &s).inside_ltv {
            panic_with_error!(&env, MarketError::Undercollateralized);
        }
        token::Client::new(&env, &loan_asset(&env)).transfer(
            &env.current_contract_address(),
            &user,
            &amount,
        );
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 5,
            amount,
        }
        .publish(&env);
        leave(&env);
    }

    pub fn repay_for(env: Env, payer: Address, user: Address, amount: i128) -> i128 {
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        payer.require_auth();
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let mut p = position(&env, &user);
        let owed = borrowed(&env, &p, &s);
        if owed == 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        let paid = if amount > owed { owed } else { amount };
        let shares = if paid == owed {
            p.debt_shares
        } else {
            multiply_div(&env, paid, INDEX, s.borrow_index)
        };
        if shares == 0 {
            panic_with_error!(&env, MarketError::Dust);
        }
        token::Client::new(&env, &loan_asset(&env)).transfer(
            &payer,
            &env.current_contract_address(),
            &paid,
        );
        p.debt_shares -= shares;
        s.debt_shares -= shares;
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 6,
            amount: paid,
        }
        .publish(&env);
        leave(&env);
        paid
    }

    pub fn liquidate(env: Env, liquidator: Address, user: Address, requested: i128) -> i128 {
        if requested <= 0 || liquidator == user {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        liquidator.require_auth();
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let mut p = position(&env, &user);
        let status = risk(&env, &p, &s);
        if status.healthy || p.collateral <= 0 || p.debt_shares <= 0 {
            panic_with_error!(&env, MarketError::NotLiquidatable);
        }
        let owed = borrowed(&env, &p, &s);
        let close = if owed / 2 == 0 { owed } else { owed / 2 };
        let token_debt = loan_asset(&env);
        let token_col = collateral_asset(&env);
        let debt_price = price(&env, &token_debt);
        let col_price = price(&env, &token_col);
        let debt_units = units(&env, &token_debt);
        let col_units = units(&env, &token_col);
        let bonus = BPS + i128::from(cfg(&env).liquidation_bonus_bps);
        let collateral_usd = multiply_div(&env, p.collateral, col_price, col_units);
        let max_pay_usd = multiply_div(&env, collateral_usd, BPS, bonus);
        let by_collateral = multiply_div(&env, max_pay_usd, debt_units, debt_price);
        let paid = requested.min(close).min(by_collateral).min(owed);
        if paid == 0 {
            panic_with_error!(&env, MarketError::Dust);
        }
        let seize_usd = multiply_div(
            &env,
            multiply_div(&env, paid, debt_price, debt_units),
            bonus,
            BPS,
        );
        let seize = multiply_div(&env, seize_usd, col_units, col_price).min(p.collateral);
        if seize <= 0 {
            panic_with_error!(&env, MarketError::Dust);
        }
        let shares = if paid == owed {
            p.debt_shares
        } else {
            multiply_div(&env, paid, INDEX, s.borrow_index)
        };
        if shares == 0 {
            panic_with_error!(&env, MarketError::Dust);
        }
        token::Client::new(&env, &token_debt).transfer(
            &liquidator,
            &env.current_contract_address(),
            &paid,
        );
        p.debt_shares -= shares;
        s.debt_shares -= shares;
        p.collateral -= seize;
        token::Client::new(&env, &token_col).transfer(
            &env.current_contract_address(),
            &liquidator,
            &seize,
        );
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 7,
            amount: paid,
        }
        .publish(&env);
        leave(&env);
        paid
    }

    pub fn absorb_bad_debt(env: Env, user: Address) -> i128 {
        admin(&env).require_auth();
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let mut p = position(&env, &user);
        if p.collateral != 0 || p.debt_shares == 0 {
            panic_with_error!(&env, MarketError::BadDebtNotEligible);
        }
        let debt = borrowed(&env, &p, &s);
        let covered = debt.min(s.reserves);
        let loss = debt - covered;
        if loss > 0 {
            if s.supply_shares <= 0 || loss >= supply_total(&env, &s) {
                panic_with_error!(&env, MarketError::BadDebtNotEligible);
            }
            let loss_index = ceil_div(
                &env,
                checked(&env, loss.checked_mul(INDEX)),
                s.supply_shares,
            );
            if loss_index >= s.supply_index {
                panic_with_error!(&env, MarketError::BadDebtNotEligible);
            }
            s.supply_index -= loss_index;
        }
        s.reserves -= covered;
        s.debt_shares -= p.debt_shares;
        p.debt_shares = 0;
        store_position(&env, &user, &p);
        save_state(&env, &s);
        MarketAction {
            user,
            action: 8,
            amount: debt,
        }
        .publish(&env);
        leave(&env);
        debt
    }

    pub fn withdraw_reserves(env: Env, to: Address, amount: i128) {
        admin(&env).require_auth();
        if amount <= 0 {
            panic_with_error!(&env, MarketError::InvalidAmount);
        }
        enter(&env);
        accrue(&env);
        let mut s = state(&env);
        let assets = checked(&env, cash(&env).checked_add(debt_amount_total(&env, &s)));
        let claims = checked(&env, supply_total(&env, &s).checked_add(amount));
        if amount > s.reserves || cash(&env) < amount || assets < claims {
            panic_with_error!(&env, MarketError::InsufficientLiquidity);
        }
        s.reserves -= amount;
        token::Client::new(&env, &loan_asset(&env)).transfer(
            &env.current_contract_address(),
            &to,
            &amount,
        );
        save_state(&env, &s);
        leave(&env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[contract]
    struct TestOracle;

    #[contractimpl]
    impl TestOracle {
        pub fn read(env: Env, asset: Address) -> PriceQuote {
            let price = if asset == env.storage().instance().get::<_, Address>(&1u32).unwrap() {
                20_000_000
            } else {
                10_000_000
            };
            PriceQuote {
                price,
                updated_at: env.ledger().timestamp(),
            }
        }
        pub fn setup(env: Env, collateral: Address) {
            env.storage().instance().set(&1u32, &collateral);
        }
    }

    #[contract]
    struct TestRate;

    #[contractimpl]
    impl TestRate {
        pub fn borrow_rate_bps(_env: Env, _utilisation_bps: u32) -> u32 {
            500
        }
    }

    fn setup() -> (Env, Address, Address, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| li.timestamp = 1000);
        let admin = Address::generate(&env);
        let supplier = Address::generate(&env);
        let borrower = Address::generate(&env);
        let collateral_sac = env.register_stellar_asset_contract_v2(admin.clone());
        let loan_sac = env.register_stellar_asset_contract_v2(admin.clone());
        let collateral = collateral_sac.address();
        let loan = loan_sac.address();
        token::StellarAssetClient::new(&env, &collateral).mint(&borrower, &100_000_000);
        token::StellarAssetClient::new(&env, &loan).mint(&supplier, &100_000_000);
        let oracle = env.register(TestOracle, ());
        TestOracleClient::new(&env, &oracle).setup(&collateral);
        let rate = env.register(TestRate, ());
        let config = RiskConfig {
            ltv_bps: 6000,
            liquidation_bps: 7500,
            liquidation_bonus_bps: 500,
            reserve_factor_bps: 1000,
            supply_cap: 100_000_000,
            borrow_cap: 80_000_000,
        };
        let market = env.register(
            HoxoLendingMarket,
            (&admin, &admin, &oracle, &rate, &collateral, &loan, &config),
        );
        (env, market, admin, supplier, borrower, collateral, loan)
    }

    #[test]
    fn supplies_borrows_repays_and_withdraws() {
        let (env, market, admin, supplier, borrower, collateral, loan) = setup();
        let m = HoxoLendingMarketClient::new(&env, &market);
        assert!(m.paused());
        m.set_paused(&admin, &false);
        m.supply(&supplier, &50_000_000);
        m.deposit_collateral(&borrower, &20_000_000);
        m.borrow(&borrower, &10_000_000);
        assert!(m.risk_of(&borrower).healthy);
        assert_eq!(m.borrowed_of(&borrower), 10_000_000);
        assert_eq!(m.available_liquidity(), 40_000_000);
        // Interest grows only on successful ledger time advancement.
        env.ledger().with_mut(|li| li.timestamp += 31_536_000 / 10);
        m.accrue_interest();
        assert!(m.borrowed_of(&borrower) > 10_000_000);
        assert!(m.supplied_of(&supplier) > 50_000_000);
        assert!(m.market_state().reserves > 0);
        token::StellarAssetClient::new(&env, &loan).mint(&borrower, &10_000_000);
        let debt = m.borrowed_of(&borrower);
        let paid = m.repay_for(&borrower, &borrower, &debt);
        assert_eq!(paid, debt);
        assert_eq!(m.borrowed_of(&borrower), 0);
        m.withdraw_collateral(&borrower, &20_000_000);
        assert_eq!(token::Client::new(&env, &collateral).balance(&market), 0);
        m.withdraw_supply(&supplier, &50_000_000);
    }

    #[test]
    #[should_panic]
    fn borrow_enforces_ltv() {
        let (env, market, admin, supplier, borrower, _, _) = setup();
        let m = HoxoLendingMarketClient::new(&env, &market);
        m.set_paused(&admin, &false);
        m.supply(&supplier, &50_000_000);
        m.deposit_collateral(&borrower, &10_000_000);
        // Collateral $20, LTV 60% = $12; borrowing $18 exceeds it.
        m.borrow(&borrower, &18_000_000);
    }

    #[test]
    #[should_panic]
    fn cannot_remove_debt_backing_collateral() {
        let (env, market, admin, supplier, borrower, _, _) = setup();
        let m = HoxoLendingMarketClient::new(&env, &market);
        m.set_paused(&admin, &false);
        m.supply(&supplier, &50_000_000);
        m.deposit_collateral(&borrower, &20_000_000);
        m.borrow(&borrower, &10_000_000);
        m.withdraw_collateral(&borrower, &18_000_000);
    }

    #[test]
    fn withdraws_and_repays_while_paused() {
        let (env, market, admin, supplier, borrower, _, _) = setup();
        let m = HoxoLendingMarketClient::new(&env, &market);
        m.set_paused(&admin, &false);
        m.supply(&supplier, &50_000_000);
        m.deposit_collateral(&borrower, &20_000_000);
        m.borrow(&borrower, &10_000_000);
        m.set_paused(&admin, &true);
        m.repay_for(&borrower, &borrower, &10_000_000);
        m.withdraw_collateral(&borrower, &20_000_000);
    }
}
