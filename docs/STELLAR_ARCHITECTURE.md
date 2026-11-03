# Hoxo Lend — Native Stellar architecture

## On-chain contract families

```text
User / Freighter (wallet-approved transactions; not yet connected to UI)
                 │
                 ▼
Stellar Soroban RPC: simulate → authorize → submit → confirmed receipt
                 │
                 ├── HoxoLendingMarket (isolated collateral-token / debt-token pair)
                 │       ├── Stellar Asset Contracts (SAC) for transfers and balances
                 │       ├── HoxoPriceOracle (price USD x 10^7, 30-min maximum age)
                 │       └── HoxoInterestRateModel (kinked borrow APR)
                 │
                 ├── HoxoRwaFactory (approved issuer, reviewed Wasm hash)
                 │       └── HoxoRwaVault (loan cap, rate, maturity, default)
                 │
                 └── HoxoLendLens (read-only market and account views)
```

This diagram describes **proposed wiring and available code**, not an activated deployment.

## Market accounting

- Every Soroban market currently serves **one loan SAC plus one different collateral SAC**.
- Supply, debt and reserve accounting use checked `i128` integer arithmetic.
- Supply and borrow indices use a fixed `10^12` index. Positions retain share counts and collateral raw token units. Interest accrual uses a kinked annualized borrow rate with BPS precision, supplied share index and configurable reserves.
- Per-market caps, administrator/guardian pause, fail-closed oracle, LTV, health/liquidation thresholds and close factor are present in the market research implementation.
- Liquidations and bad debt handling are especially sensitive to rounding and price shocks and are **not independently validated**.
- This is **not yet** a cross-collateral, 32-market pooled protocol. Cross-market health-factor aggregation and mature multi-asset portfolio management remain separate work.

## Stellar network and assets

- Network passphrase is `Test SDF Network ; September 2015`.
- RPC: `https://soroban-testnet.stellar.org`.
- Native XLM and issuer-backed assets use Stellar Asset Contract representations. Their contract IDs, issuer, decimals and trust requirements must be independently verified.
- Legacy KUB EVM addresses, `eth_call`, MetaMask and Solidity deployment workflows are never part of the active integration.
- Never expose any user secret, seed phrase, keeper secret or signing material in the frontend.

## User journey (after verification)

1. The website discovers a genuinely deployed, approved contract ID and token metadata from the canonical registry.
2. A Freighter user explicitly approves account connection on the **Stellar Testnet** network.
3. The client prepares the Soroban transaction, independently simulates and displays actual amounts, authorization tree and fees.
4. The user approves the transaction in Freighter. The client verifies returned signed XDR and contract address.
5. A backend or client broadcasts signed XDR, polls RPC and displays only confirmed transaction hashes and ledger changes.
6. The read-only lens and event indexer corroborate account balances, debt and pool solvency.

**None of steps 1–6 are represented as live in the static application today.**

## Security and compatibility

Stellar Protocol 30 Testnet upgrade was scheduled for 3 November 2026 at 17:00 UTC. SDK v29 Rust contracts require a reviewed compatibility check with the active network and Stellar CLI; JS integrations should use a current Protocol 30-capable SDK (v17 recommended). Add muxed contract address handling if needed. Do not deploy live until this review and the acceptance matrix are complete.
