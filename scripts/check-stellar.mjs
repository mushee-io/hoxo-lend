import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const cfg = JSON.parse(readFileSync("config/stellar-testnet.json","utf8"));
assert.equal(cfg.network, "Stellar Testnet");
assert.equal(cfg.passphrase, "Test SDF Network ; September 2015");
assert.equal(cfg.lendingEnabled, false, "Do not turn on unaudited lending");
assert.equal(cfg.tradingEnabled, false);
assert.deepEqual(cfg.supportedAssets, []);
assert.ok(["not-deployed","deployed-paused"].includes(cfg.deploymentStatus));

const id = /^C[A-Z2-7]{55}$/;
if (cfg.deploymentStatus === "not-deployed") {
  assert.ok(Object.values(cfg.contracts).every(x => x === null));
} else {
  for (const key of ["vault","market","oracle","interestRateModel",
                      "rwaVaultFactory","rwaVault","lens"]) {
    assert.ok(id.test(cfg.contracts[key]), key + " requires a real contract ID");
  }
  assert.equal(cfg.deploymentEvidence?.network,"Stellar Testnet");
  assert.equal(cfg.deploymentEvidence?.allContractsPaused,true);
  assert.match(cfg.deploymentEvidence?.runUrl || "",
    /^https:\/\/github\.com\/mushee-io\/hoxo-lend\/actions\/runs\/\d+$/);
}
assert.deepEqual(cfg.plannedMarkets.map(x => x.symbol), ["XLM","USDC"]);
assert.ok(cfg.plannedMarkets.every(x => !x.active && !x.marketContractId && !x.sacContractId));

const pages=["app/index.html","app/markets/index.html","app/markets/xlm/index.html",
  "app/markets/usdc/index.html","app/borrow/index.html","app/liquidations/index.html",
  "app/rwa/index.html","app/portfolio/index.html","app/protocol/index.html",
  "app/readiness/index.html","app/lab/index.html"];
for (const page of pages) {
  const content = readFileSync(page,"utf8");
  assert.match(content,/Stellar Testnet/i,page);
  assert.match(content,/not deployed|not live/i,page);
  assert.match(content,/\/src\/stellar\.js/i,page);
  assert.doesNotMatch(content,/\$2(?:\.00)?M|2000000|\$2,000,000/i,page);
}
for (const page of [...pages,"index.html","src/stellar.js"]) {
  const content=readFileSync(page,"utf8");
  assert.doesNotMatch(content,
    /bitkub|WtKUB|KUB Testnet|25925|eth_sendTransaction|window\.ethereum|mUSDT|mUSDC|0x[a-fA-F0-9]{40}/i,page);
}
console.log("Hoxo Lend Stellar-only Testnet deployment registry checks passed.");
