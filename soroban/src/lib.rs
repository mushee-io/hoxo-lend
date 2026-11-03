#![no_std]

// Hoxo Lend Milestone 1: single-asset custody / supply principal.
// This is not yet a money market: borrowing, accrual and liquidation are absent.
// Do not deploy this prototype to mainnet or custody production funds.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, Env,
};

const TTL_THRESHOLD: u32 = 518_400; // ~30 days, assuming ~5-second ledgers
const TTL_BUMP: u32 = 1_036_800; // ~60 days; retention and restore need further testing

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Asset,
    Paused,
    Total,
    Principal(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    InvalidAmount = 1,
    Paused = 2,
    InsufficientPrincipal = 3,
    ArithmeticOverflow = 4,
}

#[contractevent]
pub struct Deposited {
    pub from: Address,
    pub amount: i128,
}

#[contractevent]
pub struct Withdrawn {
    pub to: Address,
    pub amount: i128,
}

#[contract]
pub struct HoxoLendVault;

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

fn principal(env: &Env, user: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::Principal(user.clone()))
        .unwrap_or(0)
}

fn total(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::Total).unwrap_or(0)
}

#[contractimpl]
impl HoxoLendVault {
    pub fn __constructor(env: Env, admin: Address, asset: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Asset, &asset);
        env.storage().instance().set(&DataKey::Paused, &true);
        env.storage().instance().set(&DataKey::Total, &0_i128);
        bump(&env);
    }

    pub fn admin(env: Env) -> Address {
        bump(&env);
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn asset(env: Env) -> Address {
        bump(&env);
        env.storage().instance().get(&DataKey::Asset).unwrap()
    }

    pub fn paused(env: Env) -> bool {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(true)
    }

    pub fn set_paused(env: Env, paused: bool) {
        Self::admin(env.clone()).require_auth();
        env.storage().instance().set(&DataKey::Paused, &paused);
        bump(&env);
    }

    pub fn principal_of(env: Env, user: Address) -> i128 {
        bump(&env);
        principal(&env, &user)
    }

    pub fn total_supplied(env: Env) -> i128 {
        bump(&env);
        total(&env)
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidAmount);
        }
        if Self::paused(env.clone()) {
            panic_with_error!(&env, VaultError::Paused);
        }
        from.require_auth();

        let updated = principal(&env, &from)
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, VaultError::ArithmeticOverflow));
        let updated_total = total(&env)
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, VaultError::ArithmeticOverflow));

        // The token contract transfers atomically; any failure rolls back accounting.
        token::Client::new(&env, &Self::asset(env.clone())).transfer(
            &from,
            &env.current_contract_address(),
            &amount,
        );

        env.storage()
            .instance()
            .set(&DataKey::Principal(from.clone()), &updated);
        env.storage()
            .instance()
            .set(&DataKey::Total, &updated_total);
        bump(&env);
        Deposited { from, amount }.publish(&env);
    }

    pub fn withdraw(env: Env, to: Address, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, VaultError::InvalidAmount);
        }
        to.require_auth();

        let balance = principal(&env, &to);
        if balance < amount {
            panic_with_error!(&env, VaultError::InsufficientPrincipal);
        }
        let new_total = total(&env)
            .checked_sub(amount)
            .unwrap_or_else(|| panic_with_error!(&env, VaultError::ArithmeticOverflow));

        // Allow withdrawals while paused; pause only blocks fresh deposits.
        env.storage()
            .instance()
            .set(&DataKey::Principal(to.clone()), &(balance - amount));
        env.storage().instance().set(&DataKey::Total, &new_total);
        token::Client::new(&env, &Self::asset(env.clone())).transfer(
            &env.current_contract_address(),
            &to,
            &amount,
        );
        bump(&env);
        Withdrawn { to, amount }.publish(&env);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn deposits_and_withdrawals_preserve_principal() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let other = Address::generate(&env);

        let asset = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = asset.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_address);
        let token = token::Client::new(&env, &token_address);
        token_admin.mint(&user, &1_000);

        let contract = env.register(HoxoLendVault, (&admin, &token_address));
        let vault = HoxoLendVaultClient::new(&env, &contract);

        assert!(vault.paused());
        vault.set_paused(&false);
        vault.deposit(&user, &450);

        assert_eq!(vault.principal_of(&user), 450);
        assert_eq!(vault.total_supplied(), 450);
        assert_eq!(token.balance(&contract), 450);
        assert_eq!(token.balance(&user), 550);
        assert_eq!(vault.principal_of(&other), 0);

        vault.set_paused(&true);
        // Existing users may always withdraw, even during an emergency pause.
        vault.withdraw(&user, &200);
        assert_eq!(vault.principal_of(&user), 250);
        assert_eq!(vault.total_supplied(), 250);
        assert_eq!(token.balance(&contract), 250);
        assert_eq!(token.balance(&user), 750);
    }

    #[test]
    #[should_panic]
    fn rejects_deposit_while_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(admin.clone());
        let contract = env.register(HoxoLendVault, (&admin, &asset.address()));
        let vault = HoxoLendVaultClient::new(&env, &contract);
        vault.deposit(&user, &10);
    }

    #[test]
    #[should_panic]
    fn rejects_over_withdrawal() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(admin.clone());
        let contract = env.register(HoxoLendVault, (&admin, &asset.address()));
        let vault = HoxoLendVaultClient::new(&env, &contract);
        vault.withdraw(&user, &1);
    }

    #[test]
    #[should_panic]
    fn rejects_zero_deposit() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(admin.clone());
        let contract = env.register(HoxoLendVault, (&admin, &asset.address()));
        let vault = HoxoLendVaultClient::new(&env, &contract);
        vault.set_paused(&false);
        vault.deposit(&user, &0);
    }
}
