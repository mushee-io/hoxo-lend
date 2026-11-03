# Soroban port under review — NOT LIVE

Hoxo Lend maintains separate Stellar-native Soroban/Rust contract modules adapted from Maddeth's lending design. The original KUB/EVM implementation remains frozen under `legacy/maddeth-kub/` and is never deployed by the active application.

## Native contract packages

- `soroban/`: paused principal custody vault prototype
- `soroban/market/`: multi-asset credit market prototype with deposit/borrow/repay/withdraw, accrual/risk and liquidation code
- `soroban/oracle/`: permissioned price adapter; borrow-risk consumers must call `read_fresh` with finite maximum age
- `soroban/rate-model/`: utilization-based kinked rate model
- `soroban/lens/`: read-only account and market reporting
- `soroban/rwa/`: isolated RWA vault prototype
- `soroban/rwa-factory/`: issuer and vault registry prototype

These modules are **not deployed or audited** and must not be wired to public deposits or borrowing until full cross-contract tests pass. The application registry contains no verified contract IDs, asset IDs, issuer addresses, loan balances or APY figures.

## Required release checks

1. Pass Rust formatter, unit, state-invariant and adversarial tests; validate independent quote freshness and ownership.
2. Compile deterministic WASM with a Stellar CLI/SDK compatible with the current Testnet protocol.
3. Verify native XLM SAC and the specific issued USDC Testnet issuer plus SAC address. Never substitute KUB test tokens or unverified synthetic assets.
4. Verify collateral limits, reserve coverage, solvency after liquidations, interest accrual, all token amount scales, TTL/restore behavior and multi-contract authorizations.
5. Deploy deliberately paused contracts to Stellar Testnet using audited parameters and publish independently verified contract IDs and tx hashes.
6. Integrate Freighter preflight/transaction simulation and wallet-signed execution; verify contract effects and receipts.
7. Obtain independent security review and explicit public-beta signoff.

No claimed $2 million of inherited liquidity may be reused. Any live TVL, rate, user position, or contract status must come solely from Hoxo Lend's verified Stellar source.
