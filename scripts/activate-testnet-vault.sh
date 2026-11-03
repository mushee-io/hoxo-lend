#!/usr/bin/env bash
# Deploy owner-controlled XLM Stellar TESTNET custody vault. Signing secrets
# are NEVER included in the repo, CI outputs, or website.
set -euo pipefail
umask 077
: "$GITHUB_WORKSPACE" "$RUNNER_TEMP" "$GITHUB_RUN_ID"
WORK="$GITHUB_WORKSPACE"
ADMIN="$HOXO_ADMIN_PUBLIC_KEY"
ACK="$HOXO_CONFIRM_TESTNET_ONLY"
# A user-authorized change to this explicit public-only config can also trigger
# a GitHub Actions deployment; no operator secret is ever committed.
if [[ -z "$ADMIN" && -f "$WORK/config/testnet-activation-request.json" ]]; then
  ADMIN="$(python3 - "$WORK/config/testnet-activation-request.json" <<'PY'
import json,sys
with open(sys.argv[1]) as f: j=json.load(f)
print(j.get("adminPublicKey",""))
PY
)"
  ACK="$(python3 - "$WORK/config/testnet-activation-request.json" <<'PY'
import json,sys
with open(sys.argv[1]) as f: j=json.load(f)
print(j.get("confirmation",""))
PY
)"
fi
[[ "$ADMIN" =~ ^G[A-Z2-7]{55}$ ]] || { echo "A public Stellar G-address is required"; exit 1; }
[[ "$ACK" == "DEPLOY_XLM_TESTNET_VAULT" ]] || { echo "Missing explicit Testnet confirmation"; exit 1; }
OUT="$WORK/out/controlled-pilot"
mkdir -p "$OUT"
export XDG_CONFIG_HOME="$RUNNER_TEMP/hoxo-identities"
NETWORK=hoxo-controlled-testnet
SOURCE=hoxo-testnet-fee-payer
WASM="$CARGO_TARGET_DIR/wasm32v1-none/release/hoxo_lend_vault.wasm"
stellar network add "$NETWORK" --rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"
stellar network health --network "$NETWORK"
stellar keys generate "$SOURCE" --fund --network "$NETWORK"
DEPLOYER="$(stellar keys public-key "$SOURCE")"
[[ "$DEPLOYER" != "$ADMIN" ]] || exit 1
test -s "$WASM" || { echo "Missing compiled Soroban vault Wasm"; exit 1; }
XLM_SAC="$(stellar contract id asset --asset native --network "$NETWORK")"
[[ "$XLM_SAC" =~ ^C[A-Z2-7]{55}$ ]] || exit 1

RESPONSE="$(stellar contract deploy --wasm "$WASM" --network "$NETWORK" --source-account "$SOURCE" -- --admin "$ADMIN" --asset "$XLM_SAC" 2>&1)" || { echo "$RESPONSE" >&2; exit 1; }
VAULT="$(printf '%s\n' "$RESPONSE" | grep -Eo 'C[A-Z2-7]{55}' | tail -n 1 || true)"
[[ "$VAULT" =~ ^C[A-Z2-7]{55}$ ]] || { echo "Contract deployment failed: $RESPONSE"; exit 1; }

check(){
 stellar contract invoke --network "$NETWORK" --contract-id "$VAULT" --source-account "$SOURCE" --send no -- "$1" | tr -d '[:space:]"'
}
[[ "$(check admin)" == "$ADMIN" ]] || { echo "On-chain admin does not match configured public key"; exit 1; }
[[ "$(check asset)" == "$XLM_SAC" ]] || { echo "On-chain asset is not native XLM SAC"; exit 1; }
[[ "$(check paused)" == "true" ]] || { echo "New vault must deploy paused until user approves activation"; exit 1; }

python3 - "$WORK/config/stellar-testnet.json" "$OUT/deployment.json" "$ADMIN" "$VAULT" "$XLM_SAC" <<'PY'
import json,datetime,os,sys
path,proof_path,admin,vault,asset=sys.argv[1:]
with open(path) as f: cfg=json.load(f)
assert cfg["network"]=="Stellar Testnet" and cfg["lendingEnabled"] is False
proof={"network":"Stellar Testnet","vaultContractId":vault,
"adminPublicKey":admin,"nativeXlmSac":asset,"deployedPaused":True,
"verifiedOnChain":True,"deployedAt":datetime.datetime.now(datetime.timezone.utc).isoformat(),
"deploymentRunUrl":"https://github.com/mushee-io/hoxo-lend/actions/runs/"+os.environ["GITHUB_RUN_ID"],
"borrowMarketLive":False}
with open(proof_path,"w") as f: json.dump(proof,f,indent=2)
cfg["activation"]={
 "network":"Stellar Testnet","kind":"controlled-pilot","asset":"native",
 "adminPublicKey":admin,"vaultContractId":vault,
 "vaultDeploymentVerified":True,"userApprovedActivation":True,
 "vaultOperationsEnabled":True,
 "marketOperationsEnabled":False,"marketRiskVerified":False,
 "marketContractId":None,"deploymentRunUrl":proof["deploymentRunUrl"],
 "notice":"New separately administered XLM Testnet vault. Owner Freighter signature is required to unpause. Borrowing is not enabled."
}
cfg["disclaimer"]="Seven original contracts stay paused. A new owner-administered XLM Testnet vault is deployed initially paused; user must explicitly sign activation. Not audited. Borrowing remains disabled."
with open(path,"w") as f: json.dump(cfg,f,indent=2)
PY
cat "$OUT/deployment.json"
echo "Connect Freighter account $ADMIN to activate this Testnet vault: $VAULT"
