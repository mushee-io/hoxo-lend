# Hoxo Lend

**A Stellar-native lending protocol, adapted from the Maddeth product.**

This is the independent Stellar-only successor to the KUB Chain Maddeth design. The original [Maddeth repository](https://github.com/mushee-io/Maddeth) is untouched. Its full code snapshot is retained under `legacy/maddeth-kub/` strictly for design, accounting, test and product-reference purposes — **nothing in that directory is deployable as Stellar Soroban code**.

## Current status — foundation, NOT public lending

- Stellar Testnet configuration and an honest read-only website.
- A Soroban Rust single-asset **custody-vault prototype** with deposits, withdrawals, per-user principal, administrator pause and authorization checks.
- Supply/withdrawals are only implemented at contract-prototype level. No production deployment or audit is claimed.
- **Borrow, repay, accrued interest, oracle-based collateral checks, liquidations, multi-asset markets, RWA credit, Freighter signing and the live application are not implemented.**
- All user transaction controls intentionally remain disabled, and all TVL, APR and wallet metrics are unavailable rather than invented.

For porting order, risk gates and the feature-by-feature mapping from Maddeth see [docs/PORTING_PLAN.md](docs/PORTING_PLAN.md).

## Stellar application routes

The Hoxo Lend app preserves separate Dashboard, XLM/USDC Markets, Supply/Borrow, Liquidations, RWA, Portfolio, Protocol and Readiness views. All are read-only or disabled until verified Stellar deployment; planned asset listings are **not** active pools. The original KUB UI remains archived in `legacy/maddeth-kub/`.

## Local website

Run `python3 -m http.server 8080` at the repository root, then open `http://localhost:8080/` or `http://localhost:8080/app/`.

The optional RPC health-check button only calls Stellar Testnet `getHealth` and cannot sign or send transactions.

## Soroban contract (prototype)

Install a current Rust toolchain and Stellar CLI compatible with the `soroban-sdk` version pinned in `soroban/Cargo.toml`. Then:

    cargo test --manifest-path soroban/Cargo.toml
    cargo fmt --manifest-path soroban/Cargo.toml --check
    stellar contract build --manifest-path soroban/Cargo.toml

The Stellar CLI is the supported way to build deployable Wasm for `wasm32v1-none`. **Do not deploy with real funds**. Check the live Testnet protocol version and SDK compatibility before the first deployment. The Testnet Protocol 30 upgrade is scheduled for 3 November 2026; SDK/XDR compatibility must be verified.

A deployable contract ID is deliberately **not** provided. There is no mainnet release.

## Network

- Network: Stellar **Testnet** only
- Passphrase: `Test SDF Network ; September 2015`
- Soroban RPC: https://soroban-testnet.stellar.org
- Horizon: https://horizon-testnet.stellar.org

No KUB endpoint, EVM signing or KUB-chain deployment workflow is active in the Hoxo Lend application.

## Security

See [SECURITY.md](SECURITY.md). The vault prototype is unaudited; until further testing and an external review, it must not custody funds even on mainnet.
