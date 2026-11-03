#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
};

pub const BPS: i128 = 10_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Curve {
    pub base_bps: u32,
    pub slope1_bps: u32,
    pub slope2_bps: u32,
    pub kink_bps: u32,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Owner,
    Curve,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RateError {
    InvalidCurve = 1,
    InvalidUtilisation = 2,
}

fn check(env: &Env, c: &Curve) {
    if c.kink_bps == 0
        || c.kink_bps >= 10_000
        || c.base_bps > 10_000
        || c.slope1_bps > 10_000
        || c.slope2_bps > 30_000
    {
        panic_with_error!(env, RateError::InvalidCurve);
    }
}

#[contract]
pub struct HoxoInterestRateModel;

#[contractimpl]
impl HoxoInterestRateModel {
    pub fn __constructor(env: Env, owner: Address, curve: Curve) {
        check(&env, &curve);
        env.storage().instance().set(&Key::Owner, &owner);
        env.storage().instance().set(&Key::Curve, &curve);
    }

    pub fn curve(env: Env) -> Curve {
        env.storage().instance().get(&Key::Curve).unwrap()
    }

    pub fn configure(env: Env, curve: Curve) {
        let owner: Address = env.storage().instance().get(&Key::Owner).unwrap();
        owner.require_auth();
        check(&env, &curve);
        env.storage().instance().set(&Key::Curve, &curve);
    }

    pub fn borrow_rate_bps(env: Env, utilisation_bps: u32) -> u32 {
        if utilisation_bps > 10_000 {
            panic_with_error!(&env, RateError::InvalidUtilisation);
        }
        let c = Self::curve(env);
        if utilisation_bps <= c.kink_bps {
            c.base_bps + c.slope1_bps * utilisation_bps / c.kink_bps
        } else {
            c.base_bps
                + c.slope1_bps
                + c.slope2_bps * (utilisation_bps - c.kink_bps) / (10_000 - c.kink_bps)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn kink_rate_curve_is_monotonic() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let c = Curve {
            base_bps: 200,
            slope1_bps: 800,
            slope2_bps: 3000,
            kink_bps: 8000,
        };
        let addr = env.register(HoxoInterestRateModel, (&owner, &c));
        let model = HoxoInterestRateModelClient::new(&env, &addr);
        assert_eq!(model.borrow_rate_bps(&0), 200);
        assert_eq!(model.borrow_rate_bps(&4000), 600);
        assert_eq!(model.borrow_rate_bps(&8000), 1000);
        assert_eq!(model.borrow_rate_bps(&10_000), 4000);
    }

    #[test]
    #[should_panic]
    fn rejects_invalid_curve() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let c = Curve {
            base_bps: 100,
            slope1_bps: 500,
            slope2_bps: 3000,
            kink_bps: 0,
        };
        env.register(HoxoInterestRateModel, (&owner, &c));
    }
}
