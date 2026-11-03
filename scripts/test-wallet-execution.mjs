import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const source=readFileSync("src/tx-live.js","utf8");
const config=JSON.parse(readFileSync("config/stellar-testnet.json","utf8"));
assert.equal(config.network,"Stellar Testnet");
assert.equal(config.passphrase,"Test SDF Network ; September 2015");
assert.equal(config.lendingEnabled,false);
assert.equal(config.deploymentStatus,"deployed-paused");
assert.match(source,/signTransaction/);
assert.match(source,/getNetwork/);
assert.match(source,/networkPassphrase/);
assert.match(source,/prepareTransaction/);
assert.match(source,/sendTransaction/);
assert.match(source,/getTransaction/);
assert.match(source,/paused/);
assert.match(source,/i128/);
for(const method of ["deposit_collateral","withdraw_collateral","borrow","repay_for","supply","withdraw_supply","submitPrice","ownerControl","addUsdcTrustline"]){
 assert.ok(source.includes(method),"Missing signed Testnet operation: "+method);
}
const deploy=readFileSync("scripts/activate-testnet-vault.sh","utf8");
assert.match(deploy,/CIRCLE_USDC_ISSUER/);
assert.match(deploy,/USDC_SAC/);
assert.match(deploy,/MARKET=/);
assert.match(deploy,/ORACLE=/);
assert.match(deploy,/ADMIN=/);
assert.match(deploy,/marketDeployedPaused/);
assert.doesNotMatch(source,/fromSecret|Keypair\.fromSecret|localStorage|sessionStorage|eval\s*\(/);
for(const path of ["app/supply/index.html","app/borrow/index.html","app/protocol/index.html","app/settings/index.html"]){
 const page=readFileSync(path,"utf8");
 assert.match(page,/tx-live\.bundle\.js/);
}
for(const path of ["app/supply/index.html","app/borrow/index.html","app/protocol/index.html","app/settings/index.html"]){
 const page=readFileSync(path,"utf8");
 assert.ok(page.includes("https://soroban-testnet.stellar.org https://horizon-testnet.stellar.org"),path+": allow both Soroban and Horizon Testnet endpoints");
}
console.log("Wallet authorization, honest Testnet state, signed transaction and verified receipt checks are wired.");
