# Hoxo Lend — Maddeth-to-Stellar porting plan

## Boundary

- **Maddeth (KUB)**: mushee-io/Maddeth; never modify or deploy from this repository for Stellar.
- **Hoxo Lend (Stellar)**: mushee-io/hoxo-lend; new canonical home for all Stellar work.
- `legacy/maddeth-kub/` contains a frozen copy of the Maddeth frontend, Solidity pool, interest model, oracle adapter, liquidation/RWA contracts, tests and operational docs. It is archival **reference only**, not live app code.
- Stellar has no EVM and its Soroban Rust contracts are not a recompile of Solidity. Addresses, transaction signing, token APIs, auth and the storage/TTL model must be reimplemented.

## Milestones and acceptance

1. **M1 — Native foundation (started in this PR):** Soroban Rust contract crate, admin-paused principal vault, supply/withdraw unit tests, static Stellar-only website, deployment registry with null addresses. Gate: Rust tests, build and source review.
2. **M2 — Lending market accounting:** port supply/borrow indices, deterministic interest-rate accrual, reserve factor, market supply/borrow caps, operator permissions and unit/fuzz tests; never enable borrower execution without collateral constraints.
3. **M3 — Asset + oracle infrastructure:** Stellar asset contract (SAC) integration, tested decimals, oracle freshness, timeout/failure safety, independent price verification and test-asset registry; no fabricated stablecoin issuer claims.
4. **M4 — Collateral and risk:** per-account collateral status, liquidation thresholds, conservative LTV, solvency/health-factor evaluation, borrow/withdraw simulation and bad-debt controls.
5. **M5 — Liquidation and safety:** partial liquidations, close factor, collateral-limited liquidations, reserves-first bad debt handling, pause/restore TTL policies and independent adversarial testing.
6. **M6 — Wallet/app:** Freighter connection using supported Stellar SDK, authorization-tree simulation, user-review flow, real Testnet signed transactions, receipts, balance checks and publicly verifiable evidence. Then reintroduce Maddeth-style Markets, Borrow, Portfolio and isolated RWA tabs using the Hoxo design language.
7. **M7 — Audit and beta:** independent audit, reproducible build, monitoring, safety review and approved public Testnet beta. Mainnet remains separate and gated.

## MVP core accounting constraints

- Supply `>=0`, borrow `>=0`, reserves `>=0`; all arithmetic checked.
- For each market, total debt, total deposit claims, liquid balance and reserves must reconcile after every operation.
- Reject an attempt to borrow or withdraw collateral if health constraints fail, regardless of frontend state.
- Asset transfer amounts must be native integer units with the underlying SAC decimals; never assume ERC-20 wei (18 decimals).
- Require authorization from the real account for mutations, with no unapproved execution or hidden bots.
- Prices must be valid, fresh and tied to approved feeds. Failure blocks new risk-taking.
- Testnet funds and demo rewards are not real TVL; never manufacture trade, debt or transaction hashes.

## Before deployment

Verify live Stellar network protocol, pin matching SDK and CLI, compile using `stellar contract build`, run Rust tests and threat-model contract/asset authorization and TTL restore. Testnet Protocol 30 is scheduled for 2026-11-03; do not assume an SDK version without validating the current network. No private keys, seed phrases or funded account secrets are stored.
