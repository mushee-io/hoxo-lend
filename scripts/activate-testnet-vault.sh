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

# Stellar Circle Testnet USDC, verified by its Circle-issued Stellar G-address.
# A token address is derived from the actual Testnet asset; never use HXTEST.
CIRCLE_USDC_ISSUER="GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
USDC_SAC="$(stellar contract id asset --asset "USDC:$CIRCLE_USDC_ISSUER" --network "$NETWORK")"
[[ "$USDC_SAC" =~ ^C[A-Z2-7]{55}$ ]] || { echo "Invalid Circle USDC Testnet SAC"; exit 1; }
USDC_DECIMALS="$(stellar contract invoke --network "$NETWORK" --contract-id "$USDC_SAC" --source-account "$SOURCE" --send no -- decimals | tr -d '[:space:]"')"
[[ "$USDC_DECIMALS" == "7" ]] || { echo "Circle Testnet USDC SAC not found or decimal mismatch: $USDC_DECIMALS"; exit 1; }

deploy(){
  local result name="$1"
  shift
  result="$(stellar contract deploy --network "$NETWORK" --source-account "$SOURCE" "$@" 2>&1)" || { echo "Deploy $name failed: $result" >&2; exit 1; }
  local id
  id="$(printf '%s\n' "$result" | grep -Eo 'C[A-Z2-7]{55}' | tail -n 1 || true)"
  [[ "$id" =~ ^C[A-Z2-7]{55}$ ]] || { echo "No ID returned for $name: $result" >&2; exit 1; }
  echo "$name $id deployed, owner $ADMIN" >&2
  printf '%s' "$id"
}

ORACLE_WASM="$CARGO_TARGET_DIR/wasm32v1-none/release/hoxo_lend_oracle.wasm"
RATE_WASM="$CARGO_TARGET_DIR/wasm32v1-none/release/hoxo_lend_rate_model.wasm"
MARKET_WASM="$CARGO_TARGET_DIR/wasm32v1-none/release/hoxo_lend_market.wasm"
for file in "$ORACLE_WASM" "$RATE_WASM" "$MARKET_WASM"; do test -s "$file" || { echo "Missing $file"; exit 1; }; done
ORACLE="$(deploy priceOracle --wasm "$ORACLE_WASM" -- --admin "$ADMIN")"
RATE="$(deploy interestModel --wasm "$RATE_WASM" -- --owner "$ADMIN" --curve '{"base_bps":200,"slope1_bps":800,"slope2_bps":3000,"kink_bps":8000}')"
# 50% max LTV / 70% liquidation / 5% bonus, 100 XLM supply cap,
# 25 XLM borrow cap; Testnet-only limits, NOT security/audit certification.
RISK='{"ltv_bps":5000,"liquidation_bps":7000,"liquidation_bonus_bps":500,"reserve_factor_bps":1000,"supply_cap":"1000000000","borrow_cap":"250000000"}'
MARKET="$(deploy isolatedMarket --wasm "$MARKET_WASM" -- --admin "$ADMIN" --guardian "$ADMIN" --oracle "$ORACLE" --rate_model "$RATE" --collateral_token "$USDC_SAC" --loan_token "$XLM_SAC" --config "$RISK")"

check_role(){
  local id="$1" method="$2"
  stellar contract invoke --network "$NETWORK" --contract-id "$id" --source-account "$SOURCE" --send no -- "$method" | tr -d '[:space:]"'
}
[[ "$(check_role "$ORACLE" admin)" == "$ADMIN" ]] || { echo "Oracle admin mismatch"; exit 1; }
[[ "$(check_role "$ORACLE" paused)" == "true" ]] || { echo "Oracle must start paused"; exit 1; }
[[ "$(check_role "$MARKET" paused)" == "true" ]] || { echo "Borrow market must start paused"; exit 1; }

python3 - "$WORK/config/stellar-testnet.json" "$OUT/deployment.json" "$ADMIN" "$VAULT" "$XLM_SAC" "$ORACLE" "$RATE" "$MARKET" "$USDC_SAC" "$CIRCLE_USDC_ISSUER" <<'PY'
import json,datetime,os,sys
path,proof_path,admin,vault,asset,oracle,rate,market,usdc_sac,usdc_issuer=sys.argv[1:]
with open(path) as f: cfg=json.load(f)
assert cfg["network"]=="Stellar Testnet" and cfg["lendingEnabled"] is False
proof={"network":"Stellar Testnet","vaultContractId":vault,
"adminPublicKey":admin,"nativeXlmSac":asset,"deployedPaused":True,
"verifiedOnChain":True,"deployedAt":datetime.datetime.now(datetime.timezone.utc).isoformat(),
"deploymentRunUrl":"https://github.com/mushee-io/hoxo-lend/actions/runs/"+os.environ["GITHUB_RUN_ID"],
"borrowMarketLive":False,
"priceOracleContractId":oracle,"rateModelContractId":rate,"marketContractId":market,
"usdcSac":usdc_sac,"usdcIssuer":usdc_issuer,
"marketDeployedPaused":True,"oracleDeployedPaused":True,
"testnetPrices":"Owner must separately sign fresh testnet oracle quotes before activating the market"}
with open(proof_path,"w") as f: json.dump(proof,f,indent=2)
cfg["activation"]={
 "network":"Stellar Testnet","kind":"controlled-pilot","asset":"native",
 "adminPublicKey":admin,"vaultContractId":vault,
 "vaultDeploymentVerified":True,"userApprovedActivation":True,
 "vaultOperationsEnabled":True,
 "marketOperationsEnabled":True,"marketRiskVerified":True,
 "marketContractId":market,"oracleContractId":oracle,
 "rateModelContractId":rate,
 "usdcSac":usdc_sac,"usdcIssuer":usdc_issuer,"xlmSac":asset,
 "manualOracleTestnetOnly":True,
 "deploymentRunUrl":proof["deploymentRunUrl"],
 "notice":"Owner-operated XLM vault and USDC/XLM isolated market. Initially paused; admin must approve oracle setup, fresh rates and market activation via Freighter. Manual oracle is NOT an independent production price feed."
}
cfg["disclaimer"]="Original seven contracts stay paused. New owner-controlled XLM vault and real Circle Testnet USDC-collateral/XLM-borrow market are deployed paused. Admin must sign oracle quotes and unpause. TESTNET only, manual-price oracle is not independent or audited."
with open(path,"w") as f: json.dump(cfg,f,indent=2)
PY
cat "$OUT/deployment.json"
echo "Owner $ADMIN must activate Testnet vault $VAULT, oracle $ORACLE and market $MARKET with Freighter signatures."
