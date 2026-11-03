# Hoxo Lend

**A Stellar-native lending protocol, adapted from the Maddeth product.**

This is the independent Stellar-only successor to the KUB Chain Maddeth design. The original [Maddeth repository](https://github.com/mushee-io/Maddeth) is untouched. Its full code snapshot is retained under `legacy/maddeth-kub/` strictly for design, accounting, test and product-reference purposes — **nothing in that directory is deployable as Stellar Soroban code**.

## Owner-controlled live Testnet activation

**Execution integration is implemented:** the Supply and Borrow pages build real Soroban calls, simulate, request an explicit Freighter signature, broadcast only matching signed envelopes, verify finality and provide actual Stellar Testnet transaction receipts. The application never imports secrets or automatically broadcasts without wallet approval. The currently recorded seven Testnet deployments are paused behind a lost ephemeral administrator and must not be relabeled live.

An additional GitHub Actions deployment for a **new controllable Testnet pilot** is available at [Deploy controlled XLM/USDC market](https://github.com/mushee-io/hoxo-lend/actions/workflows/activate-testnet-vault.yml). It deploys **four** new owner-administered Soroban contracts: an XLM custody vault, a user-updated Testnet price oracle, an interest-rate model and a USDC-collateral / XLM-debt isolated market. It uses the genuine Circle Testnet USDC issuer `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` and derives its SAC at deployment time. This pilot is initially paused; it cannot be activated by the temporary CI fee-payer.

1. Use a **funded Stellar Testnet Freighter public G-address** you control as `admin_public_key` in the GitHub workflow; type `DEPLOY_XLM_TESTNET_VAULT` in the confirmation field. **Never** provide a secret key or recovery phrase. The workflow can also be invoked by committing an explicit config/testnet-activation-request.json containing the public admin address and confirmation string.
2. Verify the successful deployment run and the new `activation` registry entries. The workflow must confirm on-chain administrator, token identity and initial pause flags before recording addresses. It does not claim transactions have occurred.
3. Open [Supply](/app/supply/) using the same Freighter wallet on Testnet, click **Approve activation in Freighter** to sign the XLM vault `set_paused(false)`, and approve a small deposit and withdrawal. Check the actual explorer receipts.
4. For money-market supply and borrowing, open [Protocol](/app/protocol/) as the administrator; sign the Testnet oracle unpause, **enter independently checked USD prices for XLM and USDC**, sign both fresh quotes and only then sign market unpause. Manual Testnet prices **expire after 30 minutes** for new borrowing and are not independent oracle infrastructure.
5. On [Supply](/app/supply/) provide native Testnet XLM to the newly deployed lending market. On [Borrow](/app/borrow/) use Freighter to create the official Circle Testnet USDC trustline, obtain valueless USDC from [Circle's faucet](https://faucet.circle.com/), deposit USDC collateral, borrow a small amount of XLM and repay. Check each confirmed transaction hash on Stellar Testnet.

**Safety limits:** Market caps 100 Testnet XLM supplied and 25 Testnet XLM borrowed, max collateral LTV 50%, liquidation threshold 70%. This is a **manual-oracle, unaudited, valueless Testnet pilot**, not permission to accept mainnet assets or advertise production-grade yields. The current public dashboard cannot report a live lending market until the owner-controlled deployment and wallet-signed activation have produced on-chain evidence.

## Current status — deployed and paused on Stellar Testnet

Seven native Soroban contract instances were deployed and verified on 3 November 2026: lending market, custody vault, oracle, interest-rate model, isolated RWA vault, RWA issuer factory and read-only lens. Contract IDs and the successful verification run are recorded in [config/stellar-testnet.json](config/stellar-testnet.json). [On-chain deployment evidence](https://github.com/mushee-io/hoxo-lend/actions/runs/41691876395).

**No public lending is enabled.** All money-moving contract instances are paused, the frontend remains read-only, and no APY, TVL, asset balances or trade volume are claimed. The isolated market uses a throwaway HXTEST asset, not Circle USDC. No public XLM/USDC credit markets are active.

**Administrator:** This proof deployment used an ephemeral CI identity. Its secret was not saved, so the deployed admin-controlled contracts cannot be reopened by a team-controlled wallet. A new, properly governed and verified deployment is required before accepting Testnet user funds, plus a formal security/risk review and wallet-signed transaction acceptance.

For porting order, risk gates and the feature-by-feature mapping from Maddeth see [docs/PORTING_PLAN.md](docs/PORTING_PLAN.md).

## Stellar application routes

The Hoxo Lend app preserves separate Dashboard, XLM/USDC Markets, Supply/Borrow, Liquidations, RWA, Portfolio, Protocol and Readiness views. All are read-only or disabled until verified Stellar deployment; planned asset listings are **not** active pools. The original KUB UI remains archived in `legacy/maddeth-kub/`.

## Local website

Run `python3 -m http.server 8080` at the repository root, then open `http://localhost:8080/` or `http://localhost:8080/app/`.

The browser now contains wallet-approved signing and broadcasting for independently deployed owner-controlled Testnet pilots. Existing ephemeral-admin deployments remain read-only, and user transactions cannot be submitted without new verified deployment and wallet activation.

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
