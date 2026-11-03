# Hoxo Lend — Stellar Soroban security and acceptance

**Status:** This repository contains experimental native Soroban contracts and a read-only website. All components are unaudited and **no Hoxo Lend contract has been independently verified as deployed on Stellar Testnet or Mainnet**. The original Maddeth KUB implementation is archived for reference and cannot be deployed as Stellar Rust.

## Native trust boundaries

| Surface | Risk / control | Required validation |
| --- | --- | --- |
| Market `soroban/market` | Isolated pair, SAC custody, borrow shares, interest indices, LTV/liquidation thresholds | Liquidity and solvency invariants, rounding economics, malicious token/price cases |
| Oracle `soroban/oracle` | Permissioned feeder can change the price and hence liquidations | Independent price sources, circuit breakers, oracle aggregation, stale/future-price tests, monitored feeds |
| Rates `soroban/rate-model` | Administrator controls kink curve and allowed rate ranges | Tests across 0–100% utilization and extreme elapsed times |
| RWA `soroban/rwa` | Owner and borrower may originate undercollateralized credit; on-chain data cannot guarantee real-world legal claims | Real issuer verification, legal rights, servicing, permissions, tenor/default controls |
| RWA factory `soroban/rwa-factory` | Owner selects deployable Wasm hash and issuer allowlist | Verified hash, deploy/constructor tests, signed approval and governance timelocks |
| Lens `soroban/lens` | View-only snapshots may be stale until accrual and do not independently validate feeder prices | Consistent ledger snapshots and completeness across listed markets |
| Basic vault `soroban/` | Principal custody without interest, risk or loans | Migration strategy; never confuse it with a fully-fledged lending market |
| Website | Read-only; contracts null and transaction buttons disabled | Freighter simulation and auth-tree inspection before any signing feature is activated |

## Blocking issues before live funds

- Exact Stellar CLI Wasm builds under supported SDK and Protocol 30-compatible tooling. Running native `cargo test` or `cargo check` alone is **not** deployment proof.
- Native Stellar Asset Contract addresses and verified test-token issuers. The chain has 7-decimal XLM, but issued assets must use their actual SAC decimals; never assume ERC-20 18 decimals.
- Full lending accounting, independent oracle and cross-contract integration tests with signed Freighter wallets.
- Fuzz/property/invariant tests for insolvency, reserves, liquidation caps, fee/interest rounding, flash-price changes, stale feeds, auth roles, failed token transfers, re-entrancy and transaction atomicity.
- Storage TTL/restore/migration testing. Instance and persistent entries can expire; users must not lose access to funds from lapsed TTL.
- Reproducible Wasm output and reviewed upgrade/administration permissions.
- Independent external audit and security signoff. The frontend and any public deployment remain read-only/paused until then.

## Emergency response

Market and vault contracts default to paused. The lending market guardian can pause new risk-taking, but only the market administrator can reopen it. Intended emergency operations permit repayments and safe withdrawals while paused. These mechanics must be verified against real Stellar Testnet contracts before relying on them.

Do not commit, paste or send wallet secret keys. Report vulnerabilities privately to the repository maintainers.
