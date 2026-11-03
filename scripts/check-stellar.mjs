import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const config = JSON.parse(readFileSync('config/stellar-testnet.json', 'utf8'));
assert.equal(config.network, 'Stellar Testnet');
assert.equal(config.passphrase, 'Test SDF Network ; September 2015');
assert.equal(config.deploymentStatus, 'not-deployed');
assert.equal(config.lendingEnabled, false);
assert.equal(config.tradingEnabled, false);
assert.equal(config.contracts.vault, null);
assert.deepEqual(config.supportedAssets, []);

for (const file of ['index.html', 'app/index.html', 'app/markets/index.html', 'app/borrow/index.html', 'app/portfolio/index.html', 'app/rwa/index.html', 'app/protocol/index.html', 'src/stellar.js']) {
  const content = readFileSync(file, 'utf8');
  assert.doesNotMatch(content, /bitkub|KUB Testnet|25925|eth_sendTransaction|window\.ethereum/i, file);
}
for (const file of ['index.html', 'app/index.html', 'app/markets/index.html', 'app/borrow/index.html', 'app/portfolio/index.html', 'app/rwa/index.html', 'app/protocol/index.html']) {
  const content = readFileSync(file, 'utf8');
  assert.match(content, /Stellar Testnet/i, file);
  assert.match(content, /not deployed|not live/i, file);
}
console.log('Stellar-only network, safe placeholder configuration, and read-only UI checks passed.');
