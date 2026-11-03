import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const config = JSON.parse(readFileSync('config/stellar-testnet.json', 'utf8'));
assert.equal(config.network, 'Stellar Testnet');
assert.equal(config.passphrase, 'Test SDF Network ; September 2015');
assert.equal(config.deploymentStatus, 'not-deployed');
assert.equal(config.lendingEnabled, false);
assert.equal(config.tradingEnabled, false);
assert.equal(config.contracts.vault, null);
assert.equal(config.contracts.pool, null);
assert.deepEqual(config.supportedAssets, []);
assert.deepEqual(config.plannedMarkets.map(m => m.symbol), ['XLM','USDC']);
assert.ok(config.plannedMarkets.every(m => !m.active && !m.marketContractId && !m.sacContractId));

const pages = [
  'app/index.html','app/markets/index.html','app/markets/xlm/index.html',
  'app/markets/usdc/index.html','app/borrow/index.html','app/liquidations/index.html',
  'app/rwa/index.html','app/portfolio/index.html','app/protocol/index.html',
  'app/readiness/index.html','app/lab/index.html'
];
for (const file of [...pages, 'index.html', 'src/stellar.js']) {
  const content = readFileSync(file, 'utf8');
  assert.doesNotMatch(content,/bitkub|WtKUB|KUB Testnet|25925|eth_sendTransaction|window\\.ethereum|mUSDT|mUSDC|0x[a-fA-F0-9]{40}/i,file);
}
for (const file of pages) {
  const content=readFileSync(file,'utf8');
  assert.match(content,/Stellar Testnet/i,file);
  assert.match(content,/not deployed|not live/i,file);
  assert.doesNotMatch(content,/\$2(?:\.00)?M|2000000|\$2,000,000/i,file);
  assert.doesNotMatch(content,/Maddeth/i,file);
}
assert.match(readFileSync('app/markets/index.html','utf8'),/XLM/);
assert.match(readFileSync('app/markets/index.html','utf8'),/USDC/);
console.log('Hoxo Lend Stellar-only routes, asset targets and fail-closed deployment checks passed.');
