#![no_std]

// Isolated, permissioned credit-vault port from Maddeth to Soroban.
// The vault is NOT legal evidence of an RWA claim or a production lending product.
// Owner/borrower/allowlist risk and default enforcement remain critical audit items.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, Env,
};

const BPS: i128 = 10_000;
const YEAR: i128 = 31_536_000;
const MAX_MATURITY: u64 = 5 * 365 * 24 * 60 * 60;
const TTL_THRESHOLD: u32 = 518_400;
const TTL_BUMP: u32 = 1_036_800;

#[contracttype]
#[derive(Clone)]
enum Key {
    Owner,
    Borrower,
    Asset,
    Maturity,
    DebtCap,
    Paused,
    Defaulted,
    AprBps,
    Shares,
    Debt,
    Principal,
    LastAccrual,
    Allowlisted(Address),
    LenderShares(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    pub total_shares: i128,
    pub debt: i128,
    pub principal: i128,
    pub rate_bps: u32,
    pub paused: bool,
    pub defaulted: bool,
    pub maturity: u64,
    pub debt_cap: i128,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    InvalidParameter = 1,
    Paused = 2,
    NotAllowlisted = 3,
    InsufficientShares = 4,
    InsufficientCash = 5,
    BorrowLimit = 6,
    Matured = 7,
    Defaulted = 8,
    InvalidDefault = 9,
    Overflow = 10,
}

#[contractevent]
pub struct Credited {
    pub actor: Address,
    pub amount: i128,
    pub action: u32,
}

#[contract]
pub struct HoxoRwaVault;

fn keep(env: &Env) {
    env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

fn get_i(env: &Env, key: Key) -> i128 {
    env.storage().instance().get(&key).unwrap_or(0)
}

fn get_bool(env: &Env, key: Key) -> bool {
    env.storage().instance().get(&key).unwrap_or(false)
}

fn safe(env: &Env, value: Option<i128>) -> i128 {
    value.unwrap_or_else(|| panic_with_error!(env, VaultError::Overflow))
}

fn owner(env: &Env) -> Address {
    env.storage().instance().get(&Key::Owner).unwrap()
}

fn asset(env: &Env) -> Address {
    env.storage().instance().get(&Key::Asset).unwrap()
}

fn cash(env: &Env) -> i128 {
    token::Client::new(env, &asset(env)).balance(&env.current_contract_address())
}

fn lender_shares(env: &Env, user: &Address) -> i128 {
    let key = Key::LenderShares(user.clone());
    let value = env.storage().persistent().get(&key).unwrap_or(0);
    if env.storage().persistent().has(&key) {
        env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    }
    value
}

fn save_lender_shares(env: &Env, user: &Address, shares: i128) {
    let key = Key::LenderShares(user.clone());
    env.storage().persistent().set(&key, &shares);
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
}

fn accrue(env: &Env) {
    let now = env.ledger().timestamp();
    let previous: u64 = env.storage().instance().get(&Key::LastAccrual).unwrap();
    if now <= previous {
        return;
    }
    let debt = get_i(env, Key::Debt);
    let rate: u32 = env.storage().instance().get(&Key::AprBps).unwrap_or(0);
    if debt > 0 && rate > 0 {
        // Interest is simple, linear per accrual period, with compound effects
        // when a new interval begins. Rounding and accrual cadence require review.
        let numerator = safe(env, debt.checked_mul(i128::from(rate)));
        let scaled = safe(env, numerator.checked_mul(i128::from(now - previous)));
        let interest = scaled / BPS / YEAR;
        let debt_next = safe(env, debt.checked_add(interest));
        env.storage().instance().set(&Key::Debt, &debt_next);
    }
    env.storage().instance().set(&Key::LastAccrual, &now);
}

#[contractimpl]
impl HoxoRwaVault {
    pub fn __constructor(
        env: Env,
        owner: Address,
        borrower: Address,
        asset: Address,
        maturity: u64,
        debt_cap: i128,
    ) {
        let now = env.ledger().timestamp();
        if maturity <= now || maturity > now.saturating_add(MAX_MATURITY) || debt_cap <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        env.storage().instance().set(&Key::Owner, &owner);
        env.storage().instance().set(&Key::Borrower, &borrower);
        env.storage().instance().set(&Key::Asset, &asset);
        env.storage().instance().set(&Key::Maturity, &maturity);
        env.storage().instance().set(&Key::DebtCap, &debt_cap);
        env.storage().instance().set(&Key::Paused, &true);
        env.storage().instance().set(&Key::Defaulted, &false);
        env.storage().instance().set(&Key::AprBps, &0_u32);
        env.storage().instance().set(&Key::Shares, &0_i128);
        env.storage().instance().set(&Key::Debt, &0_i128);
        env.storage().instance().set(&Key::Principal, &0_i128);
        env.storage().instance().set(&Key::LastAccrual, &now);
        keep(&env);
    }

    pub fn owner(env: Env) -> Address {
        keep(&env);
        owner(&env)
    }

    pub fn borrower(env: Env) -> Address {
        keep(&env);
        env.storage().instance().get(&Key::Borrower).unwrap()
    }

    pub fn asset(env: Env) -> Address {
        keep(&env);
        asset(&env)
    }

    pub fn set_lender(env: Env, lender: Address, allowed: bool) {
        owner(&env).require_auth();
        let key = Key::Allowlisted(lender);
        env.storage().persistent().set(&key, &allowed);
        env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        keep(&env);
    }

    pub fn set_paused(env: Env, paused: bool) {
        owner(&env).require_auth();
        env.storage().instance().set(&Key::Paused, &paused);
        keep(&env);
    }

    pub fn set_fixed_apr(env: Env, bps: u32) {
        owner(&env).require_auth();
        if bps > 3_000 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        accrue(&env);
        env.storage().instance().set(&Key::AprBps, &bps);
        keep(&env);
    }

    pub fn state(env: Env) -> VaultState {
        keep(&env);
        let maturity: u64 = env.storage().instance().get(&Key::Maturity).unwrap();
        let cap: i128 = env.storage().instance().get(&Key::DebtCap).unwrap();
        VaultState {
            total_shares: get_i(&env, Key::Shares),
            debt: get_i(&env, Key::Debt),
            principal: get_i(&env, Key::Principal),
            rate_bps: env.storage().instance().get(&Key::AprBps).unwrap_or(0),
            paused: get_bool(&env, Key::Paused),
            defaulted: get_bool(&env, Key::Defaulted),
            maturity,
            debt_cap: cap,
        }
    }

    pub fn shares_of(env: Env, lender: Address) -> i128 {
        keep(&env);
        lender_shares(&env, &lender)
    }

    pub fn deposit(env: Env, lender: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        lender.require_auth();
        if get_bool(&env, Key::Paused) {
            panic_with_error!(&env, VaultError::Paused);
        }
        if get_bool(&env, Key::Defaulted) {
            panic_with_error!(&env, VaultError::Defaulted);
        }
        let key = Key::Allowlisted(lender.clone());
        if !env.storage().persistent().get(&key).unwrap_or(false) {
            panic_with_error!(&env, VaultError::NotAllowlisted);
        }
        accrue(&env);
        let total_shares = get_i(&env, Key::Shares);
        let assets = safe(&env, cash(&env).checked_add(get_i(&env, Key::Debt)));
        let minted = if total_shares == 0 {
            amount
        } else {
            if assets == 0 {
                panic_with_error!(&env, VaultError::InvalidParameter);
            }
            safe(&env, amount.checked_mul(total_shares)) / assets
        };
        if minted <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        let next = safe(&env, lender_shares(&env, &lender).checked_add(minted));
        let next_total = safe(&env, total_shares.checked_add(minted));
        token::Client::new(&env, &asset(&env))
            .transfer(&lender, &env.current_contract_address(), &amount);
        save_lender_shares(&env, &lender, next);
        env.storage().instance().set(&Key::Shares, &next_total);
        keep(&env);
        Credited { actor: lender, amount, action: 1 }.publish(&env);
    }

    pub fn borrow(env: Env, borrower: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        borrower.require_auth();
        let expected: Address = env.storage().instance().get(&Key::Borrower).unwrap();
        if borrower != expected || get_bool(&env, Key::Paused) {
            panic_with_error!(&env, VaultError::Paused);
        }
        if get_bool(&env, Key::Defaulted) {
            panic_with_error!(&env, VaultError::Defaulted);
        }
        let maturity: u64 = env.storage().instance().get(&Key::Maturity).unwrap();
        if env.ledger().timestamp() >= maturity {
            panic_with_error!(&env, VaultError::Matured);
        }
        accrue(&env);
        let debt = safe(&env, get_i(&env, Key::Debt).checked_add(amount));
        let cap: i128 = env.storage().instance().get(&Key::DebtCap).unwrap();
        if debt > cap {
            panic_with_error!(&env, VaultError::BorrowLimit);
        }
        if cash(&env) < amount {
            panic_with_error!(&env, VaultError::InsufficientCash);
        }
        let principal = safe(&env, get_i(&env, Key::Principal).checked_add(amount));
        env.storage().instance().set(&Key::Debt, &debt);
        env.storage().instance().set(&Key::Principal, &principal);
        token::Client::new(&env, &asset(&env))
            .transfer(&env.current_contract_address(), &borrower, &amount);
        keep(&env);
        Credited { actor: borrower, amount, action: 2 }.publish(&env);
    }

    pub fn repay(env: Env, payer: Address, amount: i128) -> i128 {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        payer.require_auth();
        accrue(&env);
        let debt = get_i(&env, Key::Debt);
        if debt == 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        let paid = if amount > debt { debt } else { amount };
        let principal = get_i(&env, Key::Principal);
        let interest_due = debt - principal;
        let repaid_principal = if paid > interest_due {
            paid - interest_due
        } else {
            0
        };
        token::Client::new(&env, &asset(&env))
            .transfer(&payer, &env.current_contract_address(), &paid);
        env.storage().instance().set(&Key::Debt, &(debt - paid));
        env.storage()
            .instance()
            .set(&Key::Principal, &(principal - repaid_principal));
        keep(&env);
        Credited { actor: payer, amount: paid, action: 3 }.publish(&env);
        paid
    }

    pub fn withdraw(env: Env, lender: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidParameter);
        }
        lender.require_auth();
        if get_bool(&env, Key::Defaulted) {
            panic_with_error!(&env, VaultError::Defaulted);
        }
        accrue(&env);
        let liquid = cash(&env);
        if amount > liquid {
            panic_with_error!(&env, VaultError::InsufficientCash);
        }
        let total_shares = get_i(&env, Key::Shares);
        let assets = safe(&env, liquid.checked_add(get_i(&env, Key::Debt)));
        if assets <= 0 || total_shares <= 0 {
            panic_with_error!(&env, VaultError::InsufficientShares);
        }
        let required = safe(&env, amount.checked_mul(total_shares));
        let burned = (required - 1) / assets + 1;
        let current = lender_shares(&env, &lender);
        if current < burned {
            panic_with_error!(&env, VaultError::InsufficientShares);
        }
        save_lender_shares(&env, &lender, current - burned);
        env.storage().instance().set(&Key::Shares, &(total_shares - burned));
        token::Client::new(&env, &asset(&env))
            .transfer(&env.current_contract_address(), &lender, &amount);
        keep(&env);
        Credited { actor: lender, amount, action: 4 }.publish(&env);
    }

    pub fn declare_default(env: Env) {
        owner(&env).require_auth();
        accrue(&env);
        let maturity: u64 = env.storage().instance().get(&Key::Maturity).unwrap();
        if env.ledger().timestamp() < maturity || get_i(&env, Key::Debt) == 0 {
            panic_with_error!(&env, VaultError::InvalidDefault);
        }
        env.storage().instance().set(&Key::Defaulted, &true);
        keep(&env);
    }

    pub fn cure_default(env: Env) {
        owner(&env).require_auth();
        accrue(&env);
        if get_i(&env, Key::Debt) != 0 {
            panic_with_error!(&env, VaultError::InvalidDefault);
        }
        env.storage().instance().set(&Key::Defaulted, &false);
        keep(&env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup() -> (Env, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| li.timestamp = 1000);
        let owner = Address::generate(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(owner.clone());
        let token_id = token.address();
        let sac = token::StellarAssetClient::new(&env, &token_id);
        sac.mint(&lender, &1_000_000);
        sac.mint(&borrower, &1_000_000);
        let contract = env.register(
            HoxoRwaVault,
            (&owner, &borrower, &token_id, &10_000u64, &500_000i128),
        );
        (env, contract, lender, borrower, token_id)
    }

    #[test]
    fn lenders_can_fund_borrow_repay_and_withdraw() {
        let (env, contract, lender, borrower, token_id) = setup();
        let v = HoxoRwaVaultClient::new(&env, &contract);
        v.set_lender(&lender, &true);
        v.set_paused(&false);
        v.deposit(&lender, &500_000);
        assert_eq!(v.shares_of(&lender), 500_000);
        v.borrow(&borrower, &200_000);
        assert_eq!(v.state().debt, 200_000);
        assert_eq!(v.state().principal, 200_000);
        assert_eq!(v.repay(&borrower, &200_000), 200_000);
        v.withdraw(&lender, &500_000);
        assert_eq!(v.shares_of(&lender), 0);
        assert_eq!(token::Client::new(&env, &token_id).balance(&contract), 0);
    }

    #[test]
    #[should_panic]
    fn only_allowlisted_lenders_can_deposit() {
        let (env, contract, lender, _, _) = setup();
        let v = HoxoRwaVaultClient::new(&env, &contract);
        v.set_paused(&false);
        v.deposit(&lender, &1000);
    }

    #[test]
    #[should_panic]
    fn borrower_cannot_exceed_vault_cap() {
        let (env, contract, lender, borrower, _) = setup();
        let v = HoxoRwaVaultClient::new(&env, &contract);
        v.set_lender(&lender, &true);
        v.set_paused(&false);
        v.deposit(&lender, &500_000);
        v.borrow(&borrower, &500_001);
    }
}
