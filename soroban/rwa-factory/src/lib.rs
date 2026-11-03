#![no_std]

// Hoxo Lend RWA vault factory, Stellar Soroban implementation.
// No assertion is made that the created vault represents enforceable RWA.
// Owner-controlled allowlist and Wasm hash require independent security review.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, BytesN, ContractExecutable, Env, String,
};

const MAX_URI: u32 = 512;
const MAX_MATURITY_SECONDS: u64 = 5 * 365 * 24 * 60 * 60;
const TTL_THRESHOLD: u32 = 518_400;
const TTL_BUMP: u32 = 1_036_800;

#[contracttype]
#[derive(Clone)]
enum Key {
    Owner,
    Paused,
    CodeHash,
    VaultCount,
    Issuer(Address),
    Vault(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultInfo {
    pub vault: Address,
    pub issuer: Address,
    pub borrower: Address,
    pub asset: Address,
    pub maturity: u64,
    pub debt_cap: i128,
    pub metadata_uri: String,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FactoryError {
    Paused = 1,
    NotApproved = 2,
    InvalidParameters = 3,
    WasmNotConfigured = 4,
    Overflow = 5,
}

#[contractevent]
pub struct VaultDeployed {
    pub issuer: Address,
    pub vault: Address,
}

#[contract]
pub struct HoxoRwaFactory;

fn keep(env: &Env) {
    env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

fn owner(env: &Env) -> Address {
    env.storage().instance().get(&Key::Owner).unwrap()
}

#[contractimpl]
impl HoxoRwaFactory {
    pub fn __constructor(env: Env, owner: Address) {
        env.storage().instance().set(&Key::Owner, &owner);
        env.storage().instance().set(&Key::Paused, &true);
        env.storage().instance().set(&Key::VaultCount, &0_u32);
        keep(&env);
    }

    pub fn owner(env: Env) -> Address {
        keep(&env);
        owner(&env)
    }

    pub fn paused(env: Env) -> bool {
        keep(&env);
        env.storage().instance().get(&Key::Paused).unwrap_or(true)
    }

    pub fn set_paused(env: Env, paused: bool) {
        owner(&env).require_auth();
        env.storage().instance().set(&Key::Paused, &paused);
        keep(&env);
    }

    pub fn set_issuer(env: Env, issuer: Address, approved: bool) {
        owner(&env).require_auth();
        let key = Key::Issuer(issuer);
        env.storage().persistent().set(&key, &approved);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        keep(&env);
    }

    pub fn approved_issuer(env: Env, issuer: Address) -> bool {
        keep(&env);
        let key = Key::Issuer(issuer);
        let approved = env.storage().persistent().get(&key).unwrap_or(false);
        if env.storage().persistent().has(&key) {
            env.storage()
                .persistent()
                .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        }
        approved
    }

    pub fn set_vault_wasm(env: Env, hash: BytesN<32>) {
        owner(&env).require_auth();
        // Uploaded Wasm must exist in the ledger; post-upload hash must be checked.
        env.storage().instance().set(&Key::CodeHash, &hash);
        keep(&env);
    }

    pub fn vault_count(env: Env) -> u32 {
        keep(&env);
        env.storage().instance().get(&Key::VaultCount).unwrap_or(0)
    }

    pub fn vault_info(env: Env, vault: Address) -> Option<VaultInfo> {
        keep(&env);
        let key = Key::Vault(vault);
        let info = env.storage().persistent().get(&key);
        if info.is_some() {
            env.storage()
                .persistent()
                .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        }
        info
    }

    pub fn create_vault(
        env: Env,
        issuer: Address,
        borrower: Address,
        liquidity_asset: Address,
        maturity: u64,
        debt_cap: i128,
        metadata_uri: String,
        salt: BytesN<32>,
    ) -> Address {
        if Self::paused(env.clone()) {
            panic_with_error!(&env, FactoryError::Paused);
        }
        issuer.require_auth();
        if !Self::approved_issuer(env.clone(), issuer.clone()) {
            panic_with_error!(&env, FactoryError::NotApproved);
        }
        if metadata_uri.len() > MAX_URI
            || metadata_uri.len() == 0
            || debt_cap <= 0
            || maturity <= env.ledger().timestamp()
            || maturity
                > env
                    .ledger()
                    .timestamp()
                    .saturating_add(MAX_MATURITY_SECONDS)
        {
            panic_with_error!(&env, FactoryError::InvalidParameters);
        }
        let decimals = token::Client::new(&env, &liquidity_asset).decimals();
        if decimals > 18 {
            panic_with_error!(&env, FactoryError::InvalidParameters);
        }
        let hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&Key::CodeHash)
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::WasmNotConfigured));
        let contract = env.deployer().with_current_contract(salt).deploy_contract(
            ContractExecutable::Wasm(hash),
            (&issuer, &borrower, &liquidity_asset, &maturity, &debt_cap),
        );
        let count: u32 = env.storage().instance().get(&Key::VaultCount).unwrap_or(0);
        let count = count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::Overflow));
        env.storage().instance().set(&Key::VaultCount, &count);
        let info = VaultInfo {
            vault: contract.clone(),
            issuer: issuer.clone(),
            borrower,
            asset: liquidity_asset,
            maturity,
            debt_cap,
            metadata_uri,
        };
        let key = Key::Vault(contract.clone());
        env.storage().persistent().set(&key, &info);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
        keep(&env);
        VaultDeployed {
            issuer,
            vault: contract.clone(),
        }
        .publish(&env);
        contract
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn owner_controls_issuer_approvals_and_creation_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let id = env.register(HoxoRwaFactory, (&admin,));
        let factory = HoxoRwaFactoryClient::new(&env, &id);
        assert!(factory.paused());
        assert_eq!(factory.vault_count(), 0);
        assert!(!factory.approved_issuer(&issuer));
        factory.set_issuer(&issuer, &true);
        assert!(factory.approved_issuer(&issuer));
        factory.set_paused(&false);
        assert!(!factory.paused());
    }
}
