# Hoxo Lend

**A Stellar-native lending protocol, adapted from the Maddeth product.**

This is the independent Stellar-only successor to the KUB Chain Maddeth design. The original [Maddeth repository](https://github.com/mushee-io/Maddeth) is untouched. Its full code snapshot is retained under `legacy/maddeth-kub/` strictly for design, accounting, test and product-reference purposes — **nothing in that directory is deployable as Stellar Soroban code**.

## Current status — deployed and paused on Stellar Testnet

Seven native Soroban contract instances were deployed and verified on 3 November 2026: lending market, custody vault, oracle, interest-rate model, isolated RWA vault, RWA issuer factory and read-only lens. Contract IDs and the successful verification run are recorded in [config/stellar-testnet.json](config/stellar-testnet.json). [On-chain deployment evidence](https://github.com/mushee-io/hoxo-lend/actions/runs/41691876395).

**No public lending is enabled.** All money-moving contract instances are paused, the frontend remains read-only, and no APY, TVL, asset balances or trade volume are claimed. The isolated market uses a throwaway HXTEST asset, not Circle USDC. No public XLM/USDC credit markets are active.

**Administrator:** This proof deployment used an ephemeral CI identity. Its secret was not saved, so the deployed admin-controlled contracts cannot be reopened by a team-controlled wallet. A new, properly governed and verified deployment is required before accepting Testnet user funds, plus a formal security/risk review and wallet-signed transaction acceptance.

For porting order, risk gates and the feature-by-feature mapping from Maddeth see [docs/PORTING_PLAN.md](docs/PORTING_PLAN.md).

## Stellar application routes

The Hoxo Lend app preserves separate Dashboard, XLM/USDC Markets, Supply/Borrow, Liquidations, RWA, Portfolio, Protocol and Readiness views. All are read-only or disabled until verified Stellar deployment; planned asset listings are **not** active pools. The original KUB UI remains archived in `legacy/maddeth-kub/`.

## Local website

Run `python3 -m http.server 8080` at the repository root, then open `http://localhost:8080/` or `http://localhost:8080/app/`.

The website displays genuine Testnet contract IDs and a read-only RPC health check. It cannot sign or send lending transactions.

## Soroban contract (prototype)

Install a current Rust toolchain and Stellar CLI compatible with the `soroban-sdk` version pinned in `soroban/Cargo.toml`. Then:

    cargo test --manifest-path soroban/Cargo.toml
    cargo fmt --manifest-path soroban/Cargo.toml --check
    stellar contract build --manifest-path soroban/Cargo.toml

The Stellar CLI is the supported way to build deployable Wasm for `wasm32v1-none`. **Do not deploy with real funds**. Check the live Testnet protocol version and SDK compatibility before the first deployment. The Testnet Protocol 30 upgrade is scheduled for 3 November 2026; SDK/XDR compatibility must be verified.

Verified Stellar Testnet contract IDs are recorded in the repository's network registry. There is no mainnet release.

## Network

- Network: Stellar **Testnet** only
- Passphrase: `Test SDF Network ; September 2015`
- Soroban RPC: https://soroban-testnet.stellar.org
- Horizon: https://horizon-testnet.stellar.org

No KUB endpoint, EVM signing or KUB-chain deployment workflow is active in the Hoxo Lend application.

## Security

See [SECURITY.md](SECURITY.md). The vault prototype is unaudited; until further testing and an external review, it must not custody funds even on mainnet.
