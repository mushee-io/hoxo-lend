#!/usr/bin/env bash
# Deploy Hoxo Lend to Stellar TESTNET, with money-moving contracts paused.
set -euo pipefail
umask 077

NETWORK=hoxo-lend-testnet
SOURCE=hoxo-testnet-ci
ROOT="$GITHUB_WORKSPACE"
OUT="$ROOT/out/testnet"
mkdir -p "$OUT"
export XDG_CONFIG_HOME="$RUNNER_TEMP/hoxo-identities"
WASM="$CARGO_TARGET_DIR/wasm32v1-none/release"
stellar network add "$NETWORK" --rpc-url "https://soroban-testnet.stellar.org" --network-passphrase "Test SDF Network ; September 2015"
stellar network health --network "$NETWORK"
stellar keys generate "$SOURCE" --fund --network "$NETWORK"
DEPLOYER="$(stellar keys public-key "$SOURCE")"
ADMIN="$DEPLOYER"
if [ -n "$HOXO_TESTNET_ADMIN_PUBLIC_KEY" ]; then ADMIN="$HOXO_TESTNET_ADMIN_PUBLIC_KEY"; fi
[[ "$DEPLOYER" =~ ^G[A-Z2-7]{55}$ ]] || exit 1
[[ "$ADMIN" =~ ^G[A-Z2-7]{55}$ ]] || exit 1
echo "Testnet deployer: $DEPLOYER"
echo "Admin: $ADMIN"
echo "If admin is the ephemeral CI deployer, this deployment is for viewing only."

python3 - "$OUT/deployment.json" "$DEPLOYER" "$ADMIN" <<'PY'
import json,sys
p,deployer,admin=sys.argv[1:]
with open(p,"w") as f: json.dump({
 "network":"Stellar Testnet","passphrase":"Test SDF Network ; September 2015",
 "rpcUrl":"https://soroban-testnet.stellar.org","status":"partial",
 "deployer":deployer,"adminPublicKey":admin,"ephemeralAdmin":deployer==admin,
 "publicLendingEnabled":False,"contracts":{},"assets":{},"checks":{}
},f,indent=2)
PY

record(){
  python3 - "$OUT/deployment.json" "$1" "$2" "$3" <<'PY'
import json,sys
p,section,key,val=sys.argv[1:]
with open(p) as f: j=json.load(f)
j[section][key]=val
with open(p,"w") as f: json.dump(j,f,indent=2)
PY
}
deploy(){
  local key="$1"
  shift
  local output
  if ! output="$(stellar contract deploy --network "$NETWORK" --source-account "$SOURCE" "$@" 2>&1)"; then
    echo "Failed to deploy $key: $output" >&2
    return 1
  fi
  local id
  id="$(printf '%s\n' "$output" | grep -Eo 'C[A-Z2-7]{55}' | tail -n 1 || true)"
  if [[ ! "$id" =~ ^C[A-Z2-7]{55}$ ]]; then
    echo "No contract ID returned for $key: $output" >&2
    return 1
  fi
  echo "Deployed $key: $id" >&2
  record contracts "$key" "$id"
  printf '%s' "$id"
}

XLM_SAC="$(stellar contract id asset --asset native --network "$NETWORK")"
[[ "$XLM_SAC" =~ ^C[A-Z2-7]{55}$ ]] || exit 1
record assets xlmSac "$XLM_SAC"

# HXTEST is an issued asset for this experiment, NOT Circle USDC.
ISSUED_ASSET="HXTEST:$DEPLOYER"
ISSUED_OUTPUT="$(stellar contract asset deploy --asset "$ISSUED_ASSET" --network "$NETWORK" --source-account "$SOURCE" 2>&1)" || { echo "$ISSUED_OUTPUT" >&2; exit 1; }
HXTEST_SAC="$(printf '%s\n' "$ISSUED_OUTPUT" | grep -Eo 'C[A-Z2-7]{55}' | tail -n 1 || true)"
[[ "$HXTEST_SAC" =~ ^C[A-Z2-7]{55}$ ]] || { echo "Asset contract deployment did not return ID: $ISSUED_OUTPUT" >&2; exit 1; }
record assets hxTestSac "$HXTEST_SAC"
record assets hxTestIssuer "$DEPLOYER"

for name in vault oracle rate_model market rwa_vault rwa_factory lens; do
  test -s "$WASM/hoxo_lend_$name.wasm" || { echo "Missing Wasm: $name" >&2; exit 1; }
done

ORACLE="$(deploy oracle --wasm "$WASM/hoxo_lend_oracle.wasm" -- --admin "$ADMIN")"
RATE="$(deploy rateModel --wasm "$WASM/hoxo_lend_rate_model.wasm" -- --owner "$ADMIN" --curve '{"base_bps":200,"slope1_bps":800,"slope2_bps":3000,"kink_bps":8000}')"
FACTORY="$(deploy rwaFactory --wasm "$WASM/hoxo_lend_rwa_factory.wasm" -- --owner "$ADMIN")"
LENS="$(deploy lens --wasm "$WASM/hoxo_lend_lens.wasm")"
VAULT="$(deploy vault --wasm "$WASM/hoxo_lend_vault.wasm" -- --admin "$ADMIN" --asset "$XLM_SAC")"
MATURITY="$(date -u -d '+30 days' +%s)"
RWA="$(deploy rwaVault --wasm "$WASM/hoxo_lend_rwa_vault.wasm" -- --owner "$ADMIN" --borrower "$DEPLOYER" --asset "$XLM_SAC" --maturity "$MATURITY" --debt_cap 1000000000)"
RISK='{"ltv_bps":5000,"liquidation_bps":7000,"liquidation_bonus_bps":500,"reserve_factor_bps":1000,"supply_cap":"10000000000","borrow_cap":"5000000000"}'
MARKET="$(deploy market --wasm "$WASM/hoxo_lend_market.wasm" -- --admin "$ADMIN" --guardian "$ADMIN" --oracle "$ORACLE" --rate_model "$RATE" --collateral_token "$HXTEST_SAC" --loan_token "$XLM_SAC" --config "$RISK")"

verify_paused(){
  local key="$1" id="$2" result
  result="$(stellar contract invoke --network "$NETWORK" --source-account "$SOURCE" --contract-id "$id" --send no -- paused)"
  result="$(printf '%s' "$result" | tr -d '[:space:]"')"
  if [ "$result" != "true" ]; then
    echo "$key must be paused; observed $result" >&2
    return 1
  fi
  record checks "$key" paused
}
verify_paused oracle "$ORACLE"
verify_paused vault "$VAULT"
verify_paused rwaFactory "$FACTORY"
verify_paused market "$MARKET"
stellar contract invoke --network "$NETWORK" --source-account "$SOURCE" --contract-id "$RWA" --send no -- state > "$OUT/rwa-state.json"
python3 - "$OUT/rwa-state.json" <<'PY'
import json,sys
with open(sys.argv[1]) as f: state=json.load(f)
assert state["paused"] is True, "RWA vault must remain paused"
PY
record checks rwaVault paused
stellar contract invoke --network "$NETWORK" --source-account "$SOURCE" --contract-id "$RATE" --send no -- curve > "$OUT/rate-curve.json"
record checks rateModel read-success

python3 - "$OUT/deployment.json" "$ROOT/config/stellar-testnet.json" <<'PY'
import json,datetime,os,sys
proof_path,config_path=sys.argv[1:]
with open(proof_path) as f: p=json.load(f)
assert set(p["contracts"])=={"oracle","rateModel","rwaFactory","lens","vault","rwaVault","market"}
assert all(p["checks"].get(x)=="paused" for x in ("oracle","vault","rwaFactory","market","rwaVault"))
p["status"]="deployed-paused"
p["verifiedAt"]=datetime.datetime.now(datetime.timezone.utc).isoformat()
with open(proof_path,"w") as f: json.dump(p,f,indent=2)
with open(config_path) as f: c=json.load(f)
c["contracts"]={
 "vault":p["contracts"]["vault"],"pool":None,"market":p["contracts"]["market"],
 "oracle":p["contracts"]["oracle"],"riskEngine":None,
 "interestRateModel":p["contracts"]["rateModel"],"rwaVaultFactory":p["contracts"]["rwaFactory"],
 "rwaVault":p["contracts"]["rwaVault"],"lens":p["contracts"]["lens"]
}
c["deploymentStatus"]="deployed-paused"
c["lendingEnabled"]=False
c["tradingEnabled"]=False
c["supportedAssets"]=[]
c["deploymentEvidence"]={
 "network":"Stellar Testnet","verifiedAt":p["verifiedAt"],
 "runUrl":"https://github.com/mushee-io/hoxo-lend/actions/runs/"+os.environ["GITHUB_RUN_ID"],
 "deployer":p["deployer"],"adminPublicKey":p["adminPublicKey"],
 "ephemeralAdmin":p["ephemeralAdmin"],"allContractsPaused":True,
 "xlmSac":p["assets"]["xlmSac"],"hxTestSac":p["assets"]["hxTestSac"],
 "hxTestIssuer":p["assets"]["hxTestIssuer"]
}
for market in c.get("plannedMarkets",[]):
 market["active"]=False
 market["marketContractId"]=None
c["disclaimer"]="Real Soroban contracts deployed on Stellar Testnet, PAUSED and UNAUDITED. HXTEST is a throwaway Testnet asset, not USDC. Public deposits and borrowing are disabled."
with open(config_path,"w") as f: json.dump(c,f,indent=2)
PY
echo "All seven on-chain contracts deployed with confirmed paused/read-only state."
cat "$OUT/deployment.json"
