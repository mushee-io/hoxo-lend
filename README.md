# Hoxo Lend

**Stellar-native lending protocol.** Independent Soroban/Rust conversion of Maddeth's pooled credit and isolated RWA product.

The [original Maddeth KUB repository](https://github.com/mushee-io/Maddeth) remains unchanged. Its original files are frozen as a reference-only snapshot in `legacy/maddeth-kub/`. No Solidity, EVM RPC or KUB/MetaMask address is used by the active Hoxo Lend frontend.

## Current source modules

| Soroban crate | Ported capabilities | Status |
|---|---|---|
| `soroban/` | Original principal-only custody/supply vault with pause | Research prototype |
| `soroban/market/` | Isolated collateral/debt SAC market, supply shares, indexed borrow debt, rate accrual, deposits/withdrawals, LTV/health, repay-for, partial liquidation, reserve factor, bad-debt writeoff, guardian pause | Research prototype |
| `soroban/rate-model/` | Kinked utilisation rate contract | Research prototype |
| `soroban/oracle/` | Native permissioned Testnet price feed and timestamp | Research prototype; not independent pricing |
| `soroban/rwa/` | Allowlisted fixed-rate isolated credit vault, loan cap, maturity, repayments, default/cure | Research prototype |
| `soroban/rwa-factory/` | Issuer allowlist and deterministic Soroban vault deployment registry | Research prototype |
| `soroban/lens/` | Read-only market, supply, debt and health aggregation | Research prototype |

**None of the above are independently audited, mainnet-safe or verified as deployed.** The canonical Stellar contract addresses are all `null`; the website does not sign or transmit transactions. No TVL, APY, contract receipts, issuer claims or asset availability are advertised as live.

## Stellar-native application

The static website uses Stellar-only terms and separate sections: `/`, `/app/`, `/app/markets/`, `/app/borrow/`, `/app/portfolio/`, `/app/rwa/`, `/app/protocol/`, `/docs/`.

- Stellar Testnet passphrase: `Test SDF Network ; September 2015`
- Soroban RPC: https://soroban-testnet.stellar.org
- Horizon: https://horizon-testnet.stellar.org
- Stellar assets are used via their native Stellar Asset Contract (SAC) IDs.
- Any Freighter signing / simulation and market registry enablement are explicitly gated until a real Testnet deployment and security review.

## Local checks

```sh
python3 -m http.server 8080
node scripts/check-stellar.mjs

for crate in soroban soroban/market soroban/oracle soroban/rate-model soroban/rwa soroban/rwa-factory soroban/lens; do
  cargo fmt --manifest-path "$crate/Cargo.toml" --check
  cargo test --manifest-path "$crate/Cargo.toml"
done
```

Use an up-to-date Stellar CLI compatible with the pinned Soroban SDK and currently activated Testnet protocol for actual Wasm builds. Do not deploy before validating contract math, SAC authorization/transfer semantics, price feeds, storage TTL/restore behavior, cross-contract data types, test coverage and economic solvency.

See [porting plan](docs/PORTING_PLAN.md), [module matrix](docs/CONVERSION_MATRIX.md) and [security gates](SECURITY.md).

## Remaining gates

- Exact Wasm compilation and SDK compatibility with the latest Stellar protocol.
- Complete multi-market pooled collateral and RWA legal/issuer integration; isolated-pair market prototypes are not a pooled multi-market protocol.
- Freighter wallet signed authorize/simulate/submit/receipt flows.
- Extensive property/invariant/fuzz tests including stale prices, malicious assets, TTL expiry, insolvent positions and borrow/repay rounding.
- Independent security audit and deployment evidence before enabling actual Testnet funds, then a separate formal mainnet go/no-go.
