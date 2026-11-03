// Read-only Stellar Testnet RPC probe. No wallet integration or transaction submission.
async function checkNetwork() {
  const label = document.getElementById('network-health');
  const button = document.getElementById('check-network');
  if (!label || !button) return;
  button.disabled = true;
  label.textContent = 'Checking Stellar Testnet RPC…';
  try {
    const configRes = await fetch('/config/stellar-testnet.json', {cache:'no-store'});
    if (!configRes.ok) throw new Error('Could not load network configuration');
    const config = await configRes.json();
    if (config.network !== 'Stellar Testnet' ||
        config.passphrase !== 'Test SDF Network ; September 2015' ||
        config.deploymentStatus !== 'not-deployed') {
      throw new Error('Unexpected network or deployment configuration');
    }
    const rpcRes = await fetch(config.rpcUrl, {
      method:'POST',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({jsonrpc:'2.0',id:1,method:'getHealth'})
    });
    if (!rpcRes.ok) throw new Error('RPC HTTP ' + rpcRes.status);
    const response = await rpcRes.json();
    if (response.error) throw new Error(response.error.message || 'RPC returned an error');
    label.textContent = response.result?.status === 'healthy'
      ? 'Stellar Testnet RPC healthy · Hoxo Lend contracts not deployed'
      : 'RPC response received · Hoxo Lend contracts not deployed';
  } catch (error) {
    label.textContent = 'RPC status unavailable · ' + (error.message || 'Network check failed');
  } finally {
    button.disabled = false;
  }
}
document.getElementById('check-network')?.addEventListener('click', checkNetwork);
