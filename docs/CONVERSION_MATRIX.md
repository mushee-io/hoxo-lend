# Hoxo Lend Stellar-native conversion matrix

**Canonical source:** legacy/maddeth-kub (untouched archive). **Target:** independent Soroban Rust packages. No legacy KUB Solidity or Foundry code is used by the active Testnet frontend.

| Original Maddeth/KUB feature | Stellar Soroban target | Current state |
| --- | --- | --- |
| MaddethPool: supply / withdraw accounting | `soroban/src/lib.rs` HoxoLendVault | Primitive single-asset principal vault only; not a money market |
| InterestRateModel kinked utilisation curve | `soroban/rate-model/src/lib.rs` | First native Soroban implementation and unit tests; integration pending |
| BitkubOracleAdapter | `soroban/oracle/src/lib.rs` | Permissioned Testnet feed contract prototype; independent data sources, controls and audit pending |
| MaddethPool markets and borrow/repay | `soroban/market/src/lib.rs` | Soroban research prototype; unit and integration CI required |
| Collateral enablement, account risk, indexed interest | Market-level risk/position modules | Isolated-pair risk checks and indexed shares; multichain/multimarket aggregation and production invariants pending |
| Partial liquidation, close factor, seize bonus | Native Soroban liquidation path | First isolated-pair liquidation method; stress, oracle and solvency tests pending |
| Reserve factor, bad-debt absorption | Market-level solvency controls | First reserve/bad-debt path; accounting audit and insolvency simulation pending |
| IsolatedRwaVault | `soroban/rwa/src/lib.rs` | Initial isolated Soroban RWA loan/vault port; high-risk and unaudited |
| RwaVaultFactory: issuer registry/factory | \`soroban/rwa-factory/src/lib.rs\` | Native issuer allowlist and deterministic Wasm deployment prototype; full integration and audits pending |
| MaddethLens | \`soroban/lens/src/lib.rs\` | First read-only Soroban aggregator, returning actual market state and account risk; RPC indexer not deployed |
| WrappedTKUB | Stellar-native XLM via Stellar Asset Contract (SAC) | Pending native asset configuration; do not wrap or rename tKUB |
| KUB test assets | Whitelisted Stellar Testnet asset contract IDs and verified issuers | No approved token IDs |
| MetaMask EIP-1193 | Freighter + Stellar SDK transaction simulation/auth | Not integrated |
| EVM eth_call / ABIs | Soroban RPC simulation and contract specifications | Not integrated |
| Forge/Slither | Rust tests, Wasm build, Soroban security reviews | Rust tests added; no formal audit |
| KUB landing and dashboards | Stellar-only Hoxo Lend site and application | Read-only site; no live onchain market |

## Porting rules

- Do not copy Solidity bytecode into Stellar deployments; reimplement semantics in audited Rust.
- Each token amount is native to its SAC decimals, and all prices must explicitly specify their scale.
- No fake rates, TVL, deployment addresses or transaction hashes.
- Hoxo Lend is Stellar-only. The KUB Maddeth repository is unchanged.
- Owner, guardians, oracle feeders and vault issuers must pass actual Stellar auth checks, not browser-side switches.
- Borrowing and collateral releases must fail closed on invalid, missing or stale prices.
- High-risk contracts are strictly research/testnet-only until invariant, fuzz, TTL restore, migration, adversarial, and independent audits pass.
