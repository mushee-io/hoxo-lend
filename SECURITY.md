# Hoxo Lend security and release controls

Hoxo Lend is **experimental Stellar Testnet software** and is **not production-ready**. No independent audit has been completed. No protocol TVL, borrowing, interest or liquidation data is advertised as live.

## Controls

- The public website is **read-only**; it cannot request a signature or broadcast a transaction.
- Soroban vault contracts begin **paused** by default. Only an administrator can unpause them.
- User deposits require explicit authorization and an amount greater than zero. Withdrawals require user authorization, sufficient recorded principal and a valid positive amount.
- Contract asset addresses are provided at initialization and cannot be changed.
- The initial vault tracks custody principal only; it has **no lending/borrow authority**, lending rates, liquidation mechanism, or oracle dependency.
- Restore/TTL, asset issuer controls, decimal precision, fee/rent economics, upgrades, front-end transaction construction, threat modelling, fuzz/invariant tests and third-party audits remain required before enabling any deposit UI or borrowing flows.
- Do not copy or reuse KUB deployments, EVM addresses, secret keys or MetaMask signing paths. The original EVM implementation exists under `legacy/` for reference only.

## Release gates

1. Verify SDK compatibility with the current Stellar Testnet protocol, especially Protocol 30.
2. Build and test Wasm with Stellar CLI; add property, authorization, overflow and denial-of-service tests.
3. Complete borrow/repay with a validated risk engine, price freshness, interest indices, caps and liquidation accounting.
4. Verify Freighter preflight/sign/submit/confirm against actual Testnet contract IDs and explorer receipts.
5. Independently review the smart contracts and close all critical findings.
6. Only then consider an opt-in public beta. Mainnet requires a separate formal go/no-go review.

Never commit secrets. To report a security issue, contact the repository maintainers privately; do not disclose exploit details in public issues.

## Verified Testnet deployment (3 November 2026)

Seven Soroban contract instances have been deployed and verified on Stellar **Testnet**, with all funds-moving contracts paused. See [`config/stellar-testnet.json`](config/stellar-testnet.json) and [deployment run](https://github.com/mushee-io/hoxo-lend/actions/runs/41691876395). The deployment administrator was an ephemeral CI identity whose secret was not retained; re-deploy under secured team ownership before testing public transactions. This is not mainnet readiness, a verified USDC listing, a completed audit, or authorization to accept public deposits.

## New owner-controlled Testnet lending pilot

The source includes a manual GitHub Actions deployment for a new administrator-owned native XLM vault and isolated Circle Testnet USDC/XLM market. The deployer fee-payer is deliberately ephemeral; the privileged administrator must be the user's **public Freighter G-address**, signed from their own wallet. The new oracle is centrally administered with manually submitted USD prices and a 30-minute freshness limit; it is not an audited or independently fed oracle. Market caps do not replace formal economic testing. No browser signing seeds or secret keys are stored. All Testnet transactions require an explicit Freighter approval, wallet-network validation, Soroban simulation, exact-envelope comparison, confirmed receipt and explorer proof. Do not move real-value assets or treat the Testnet implementation as production ready.
