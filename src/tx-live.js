/**
 * Hoxo Lend — real, wallet-approved Stellar Testnet contract transactions.
 *
 * This is NOT a synthetic activity generator. Each action:
 *   - revalidates the active chain, contract IDs, and wallet network;
 *   - reads the live Soroban paused flag and market asset configuration;
 *   - simulates an exact contract invocation;
 *   - shows the intended method, amount, contract, and maximum fee;
 *   - obtains explicit Freighter approval (no signing credentials retained);
 *   - broadcasts only the wallet-signed, hash-matching Testnet envelope;
 *   - waits for RPC SUCCESS and links to the real ledger transaction.
 *
 * Current ephemeral-admin deployments stay read-only. The registry must
 * point to a team-administered pilot before this module can enable writes.
 */
import {
  Address, Asset, BASE_FEE, Contract, Horizon, Networks, Operation,
  TransactionBuilder, nativeToScVal, rpc, scValToNative
} from "@stellar/stellar-sdk";
import { getAddress, getNetwork, requestAccess, signTransaction } from "@stellar/freighter-api";

const PASSPHRASE = "Test SDF Network ; September 2015";
const RPC = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const G = /^G[A-Z2-7]{55}$/;
const C = /^C[A-Z2-7]{55}$/;
const MAX_FEE_STROOPS = 25_000_000n;
const MAX_ACTION_STROOPS = 1_000_000_000n; // pilot transaction cap: 100 XLM
const POLL_COUNT = 18;
const rpcServer = new rpc.Server(RPC);
const horizon = new Horizon.Server(HORIZON);
let registry = null;
let sourceAccount = null;
let pending = false;

const sleep=(ms)=>new Promise(resolve=>setTimeout(resolve,ms));
function fail(message){throw new Error(message);}
const byId=(id)=>document.getElementById(id);
const status=(message,kind="info")=>{
  const area=byId("hoxo-live-status");
  if(area){area.textContent=message;area.dataset.kind=kind;}
  const toast=byId("hoxo-toast");
  if(toast){
    toast.hidden=false;
    toast.textContent=message;
  }
};

function parseAmount(input){
  const value=String(input||"").trim();
  if(!/^(?:0|[1-9]\d*)(?:\.\d{1,7})?$/.test(value)) fail("Enter an amount with at most 7 decimal places.");
  const [whole,part=""]=value.split(".");
  const stroops=BigInt(whole)*10_000_000n+BigInt(part.padEnd(7,"0"));
  if(stroops<=0n)fail("Amount must be greater than zero.");
  if(stroops>MAX_ACTION_STROOPS)fail("Pilot transactions are limited to 100 XLM per action.");
  return stroops;
}
function validateRegistry(json){
  if(json?.network!=="Stellar Testnet"||json.passphrase!==PASSPHRASE||
    json.rpcUrl!==RPC||json.horizonUrl!==HORIZON)
    fail("The configured network is not the official Stellar Testnet.");
  // The original seven proof-of-deployment instances are controlled by a lost
  // ephemeral administrator. A NEW, independently verified owner-controlled
  // pilot must be evaluated separately; it does not revive those instances.
  const pilot=json.activation;
  if(!pilot||pilot.network!=="Stellar Testnet"||pilot.kind!=="controlled-pilot"||
      !G.test(pilot.adminPublicKey||"")||!C.test(pilot.vaultContractId||"")||
      pilot.asset!=="native"||pilot.vaultDeploymentVerified!==true||
      pilot.userApprovedActivation!==true)
    return {cfg:json,pilot:false};
  return {cfg:json,pilot:true};
}
async function readRegistry(){
  const response=await fetch("/config/stellar-testnet.json",{cache:"no-store"});
  if(!response.ok)fail("Testnet deployment registry is unavailable.");
  const json=await response.json();
  const data=validateRegistry(json);
  registry=data;
  return data;
}
async function readWallet({request=false}={}){
  const value=request?await requestAccess():await getAddress();
  if(value?.error)fail(value.error?.message||"Freighter connection was rejected.");
  const publicKey=value?.address;
  if(!G.test(publicKey||""))fail("Connect a funded Stellar Testnet G-address with Freighter.");
  const network=await getNetwork();
  if(network.error||network.networkPassphrase!==PASSPHRASE)
    fail("Switch Freighter to Stellar Testnet. No transaction has been signed.");
  sourceAccount=publicKey;
  return publicKey;
}
async function readPaused(contractId){
  if(!C.test(contractId||""))fail("Unverified Soroban contract ID.");
  const wallet=await readWallet();
  const account=await rpcServer.getAccount(wallet);
  const tx=new TransactionBuilder(account,{fee:BASE_FEE,networkPassphrase:Networks.TESTNET})
    .addOperation(new Contract(contractId).call("paused")).setTimeout(50).build();
  const sim=await rpcServer.simulateTransaction(tx);
  if(sim.error||!sim.result?.retval)fail("Cannot read the on-chain paused flag.");
  const value=scValToNative(sim.result.retval);
  if(typeof value!=="boolean")fail("Unexpected paused state returned by Soroban.");
  return value;
}
function ensurePilot(mode,contractId){
  if(!registry?.pilot)fail("The verified team-controlled Testnet pilot has not been deployed.");
  const active=registry.cfg.activation;
  if(mode==="vault" && !active.vaultOperationsEnabled)fail("XLM vault pilot is not enabled in the registry.");
  if(mode==="market" && (!active.marketOperationsEnabled||!C.test(active.marketContractId||"")))
    fail("Borrowing market is not activated. Vault supply and withdrawal can be tested first.");
  if(!C.test(contractId||""))fail("No verified contract ID for this operation.");
}
async function createInvocation(address,contractId,method,args){
  const account=await rpcServer.getAccount(address);
  return new TransactionBuilder(account,{
    fee:BASE_FEE,networkPassphrase:Networks.TESTNET
  }).addOperation(new Contract(contractId).call(method,...args))
  .setTimeout(60).build();
}
async function submitExactTx({contractId,method,args,amount,description}){
  if(pending)fail("Another Testnet transaction is being processed.");
  const wallet=await readWallet({request:true});
  if(!C.test(contractId||""))fail("Contract address is not a valid Stellar C-address.");
  pending=true;
  try{
    const tx=await createInvocation(wallet,contractId,method,args);
    status("Simulating Soroban Testnet transaction…");
    const sim=await rpcServer.simulateTransaction(tx);
    if(sim.error)fail("On-chain preflight rejected the transaction: "+sim.error);
    const prepared=await rpcServer.prepareTransaction(tx);
    const fee=BigInt(prepared.fee);
    if(fee<=0n||fee>MAX_FEE_STROOPS)fail("Network fee exceeds the pilot's allowed fee cap.");
    const confirmation=[
      "HOXO LEND · STELLAR TESTNET",
      description,
      "Contract: "+contractId,
      "Method: "+method,
      "Amount: "+(amount||"—"),
      "Maximum network fee: "+(Number(fee)/10_000_000).toFixed(7)+" XLM",
      "Wallet: "+wallet,
      "",
      "Proceed to Freighter approval? No transaction is sent until you sign."
    ].join("\n");
    if(!window.confirm(confirmation)){
      status("Wallet transaction cancelled; nothing was broadcast.");
      return null;
    }
    status("Review and sign the exact Testnet transaction in Freighter…");
    const signedResult=await signTransaction(prepared.toXDR(),{
      networkPassphrase:PASSPHRASE,address:wallet,submit:false
    });
    if(signedResult?.error)fail(signedResult.error?.message||"Freighter rejected signing.");
    if(signedResult.signerAddress && signedResult.signerAddress!==wallet)
      fail("Signed wallet does not match the requested address.");
    if(!signedResult.signedTxXdr)fail("Freighter returned no signed transaction.");
    const signed=TransactionBuilder.fromXDR(signedResult.signedTxXdr,PASSPHRASE);
    if(signed.source!==wallet||signed.operations.length!==1)
      fail("Wallet returned an unexpected transaction envelope.");
    const intendedHash=prepared.hash().toString("hex");
    const signedHash=signed.hash().toString("hex");
    if(intendedHash!==signedHash)fail("The transaction envelope changed during signing.");
    if(!signed.signatures?.length)fail("Wallet returned a transaction without an envelope signature.");
    status("Submitting wallet-approved transaction to Stellar Testnet…");
    const sent=await rpcServer.sendTransaction(signed);
    if(sent.errorResult||sent.status==="ERROR"||!sent.hash)
      fail("Stellar RPC refused transaction submission.");
    const hash=sent.hash;
    if(!/^[a-f0-9]{64}$/.test(hash))fail("RPC returned an invalid transaction hash.");
    const link="https://stellar.expert/explorer/testnet/tx/"+hash;
    status("Submitted: "+hash+". Waiting for finality…");
    for(let i=0;i<POLL_COUNT;i++){
      await sleep(1900);
      const receipt=await rpcServer.getTransaction(hash);
      if(receipt.status==="SUCCESS"){
        status("Confirmed on Stellar Testnet: "+hash,"success");
        showReceipt(link,hash,description);
        window.dispatchEvent(new CustomEvent("hoxo:testnet-confirmed",{detail:{hash,contractId,method}}));
        return hash;
      }
      if(receipt.status==="FAILED")fail("Transaction failed on-chain: "+hash);
    }
    showReceipt(link,hash,"Submitted; finality not yet verified");
    status("Submitted, but final confirmation was not observed. Check the explorer: "+hash);
    return null;
  }finally{pending=false;}
}
function showReceipt(url,hash,description){
  let holder=byId("hoxo-real-receipt");
  if(!holder){
    holder=document.createElement("p");
    holder.id="hoxo-real-receipt";
    const section=byId("hoxo-live-status")?.parentNode||document.querySelector("main");
    section?.append(holder);
  }
  holder.replaceChildren();
  holder.append(document.createTextNode(description+": "));
  const link=document.createElement("a");
  link.target="_blank";
  link.rel="noopener noreferrer";
  link.href=url;
  link.textContent=hash.slice(0,12)+"…"+hash.slice(-12)+" ↗";
  holder.append(link);
}
async function signAction(mode,method,amountInput){
  if(!registry)await readRegistry();
  const pilot=registry.cfg.activation||{};
  const contractId=mode==="vault"?pilot.vaultContractId:pilot.marketContractId;
  ensurePilot(mode,contractId);
  const wallet=await readWallet({request:true});
  const paused=await readPaused(contractId);
  const moneyIn=["deposit","supply","deposit_collateral","borrow"].includes(method);
  if(paused&&moneyIn)fail("This contract is still paused on-chain; no funds have been moved.");
  // Market operations additionally require an independently validated feeder,
  // asset registry, and active risk configuration recorded in the pilot.
  if(mode==="market" && pilot.manualOracleTestnetOnly!==true)
    fail("This Testnet market has not been configured for explicit owner-controlled pilot pricing.");
  const amount=parseAmount(amountInput);
  let args;
  if(method==="repay_for")args=[new Address(wallet).toScVal(),new Address(wallet).toScVal(),nativeToScVal(amount,{type:"i128"})];
  else args=[new Address(wallet).toScVal(),nativeToScVal(amount,{type:"i128"})];
  return submitExactTx({
    contractId,method,args,amount:amountInput+(method.includes("collateral")?" USDC":" XLM"),
    description:mode==="vault"?"XLM custody vault "+method:"Lending market "+method
  });
}
async function activateVault(){
  if(!registry)await readRegistry();
  const activation=registry.cfg.activation;
  if(!registry.pilot||!activation?.vaultOperationsEnabled)
    fail("Deploy a team-owned Testnet vault before requesting activation.");
  const wallet=await readWallet({request:true});
  if(wallet!==activation.adminPublicKey)fail("Only the recorded administrator may activate the Testnet vault.");
  if(!await readPaused(activation.vaultContractId)){
    status("Vault already activated on Stellar Testnet.");
    return;
  }
  const hash=await submitExactTx({
    contractId:activation.vaultContractId,
    method:"set_paused",args:[nativeToScVal(false,{type:"bool"})],
    description:"Administrator approval: unpause XLM vault (Testnet only)"
  });
  if(hash)refreshButtons();
}
async function readSupplyAccount(){
  const wallet=await readWallet();
  if(!registry?.pilot)return;
  const pilot=registry.cfg.activation;
  const account=await horizon.loadAccount(wallet);
  const native=account.balances.find(b=>b.asset_type==="native");
  const balance=native?.balance||"—";
  const el=byId("wallet-live-balance");
  if(el)el.textContent="Wallet XLM balance: "+balance;
  return {wallet,balance};
}
async function refreshButtons(){
  const disabled=!(registry?.pilot && registry.cfg.activation?.vaultOperationsEnabled);
  const walletAction=byId("hoxo-vault-activate");
  if(walletAction)walletAction.hidden=disabled;
  const note=byId("hoxo-live-status");
  if(!note)return;
  if(disabled){
    note.textContent="A new vault with your Freighter wallet as administrator is required before live Testnet transactions.";
    return;
  }
  try{
    const paused=await readPaused(registry.cfg.activation.vaultContractId);
    note.textContent=paused
      ?"New Testnet vault deployed. Administrator must approve activation in Freighter."
      :"Live Testnet vault is accepting wallet-signed XLM deposits and withdrawals.";
    const indicator=byId("vault-live-badge");
    if(indicator)indicator.textContent=paused?"READY FOR ADMIN SIGNATURE":"LIVE TESTNET · ON-CHAIN";
    await readSupplyAccount();
  }catch(error){note.textContent="Wallet/network checks pending: "+error.message;}
}

async function ownerControl(kind){
  if(!registry)await readRegistry();
  if(!registry.pilot)fail("The new owner-operated market is not registered.");
  const pilot=registry.cfg.activation;
  const admin=await readWallet({request:true});
  if(admin!==pilot.adminPublicKey)fail("Use the owner Freighter account recorded in the Testnet deployment.");
  const contractId=kind==="oracle"?pilot.oracleContractId:pilot.marketContractId;
  if(!C.test(contractId||""))fail("No verified "+kind+" contract to activate.");
  if(!await readPaused(contractId)){status(kind+" is already unpaused on Stellar Testnet.");return;}
  if(kind==="market"){
    if(await readPaused(pilot.oracleContractId))
      fail("First activate the oracle and submit current Testnet XLM and USDC prices.");
    if(!window.confirm("TESTNET ONLY: This deploys a manually administered oracle, not an independently audited price feed. You must provide current values and refresh them at least every 30 minutes for new borrowing. Unpause testnet lending?"))return;
  }
  const method="set_paused";
  const args=kind==="oracle"
    ?[nativeToScVal(false,{type:"bool"})]
    :[new Address(admin).toScVal(),nativeToScVal(false,{type:"bool"})];
  await submitExactTx({
    contractId,method,args,
    description:kind==="oracle"?"Activate manual Testnet oracle":"Activate capped Circle USDC / XLM Testnet borrow market"
  });
  await refreshButtons();
}
async function submitPrice(token){
  if(!registry)await readRegistry();
  if(!registry.pilot)fail("No owner-controlled Testnet oracle deployment exists.");
  const pilot=registry.cfg.activation;
  const admin=await readWallet({request:true});
  if(admin!==pilot.adminPublicKey)fail("Only the owner can publish this Testnet oracle quote.");
  if(await readPaused(pilot.oracleContractId))
    fail("The owner must first unpause the oracle in Freighter.");
  const tokenAddress=token==="USDC"?pilot.usdcSac:pilot.xlmSac;
  if(!C.test(tokenAddress||""))fail("Missing verified Testnet token SAC.");
  const value=byId(token==="USDC"?"hoxo-oracle-usdc-price":"hoxo-oracle-xlm-price")?.value;
  const price=parseAmount(value);
  if(token==="USDC" && (price<8_000_000n||price>12_000_000n))
    fail("Testnet USDC quotes must be within the $0.80 to $1.20 manual safety band.");
  if(!window.confirm(
    "Manual price feed: "+token+" = $"+value+
    ". This is user-entered TESTNET pricing, not a verified independent oracle. "+
    "Do you confirm the USD value is based on a recent independently checked source?"))return;
  await submitExactTx({
    contractId:pilot.oracleContractId,method:"submit",
    args:[new Address(admin).toScVal(),new Address(tokenAddress).toScVal(),nativeToScVal(price,{type:"i128"})],
    amount:value+" USD per "+token,
    description:"Publish owner-signed manual TESTNET "+token+" oracle quote"
  });
}
async function addUsdcTrustline(){
  if(pending)fail("Another transaction is pending.");
  if(!registry)await readRegistry();
  if(!registry.pilot)fail("USDC market registry not published.");
  const pilot=registry.cfg.activation;
  if(!G.test(pilot.usdcIssuer||""))fail("Missing independently verified Circle Testnet issuer.");
  const address=await readWallet({request:true});
  const account=await horizon.loadAccount(address);
  const already=account.balances.some(b=>b.asset_code==="USDC"&&b.asset_issuer===pilot.usdcIssuer);
  if(already){status("Circle Testnet USDC trustline already exists.");return;}
  const asset=new Asset("USDC",pilot.usdcIssuer);
  const tx=new TransactionBuilder(account,{fee:BASE_FEE,networkPassphrase:PASSPHRASE})
    .addOperation(Operation.changeTrust({asset})).setTimeout(60).build();
  if(!window.confirm(
    "CREATE STELLAR TESTNET TRUSTLINE\nAsset: USDC\nCircle Testnet issuer: "+pilot.usdcIssuer+
    "\nAccount: "+address+"\n\nFreighter will request your explicit approval. This action does not borrow or deposit."))return;
  pending=true;
  try{
    const response=await signTransaction(tx.toXDR(),{
      address,networkPassphrase:PASSPHRASE,submit:false
    });
    if(response?.error||!response.signedTxXdr)fail(response?.error?.message||"Freighter rejected trustline authorization.");
    const signed=TransactionBuilder.fromXDR(response.signedTxXdr,PASSPHRASE);
    if(signed.source!==address||signed.operations.length!==1||
       signed.hash().toString("hex")!==tx.hash().toString("hex")||!signed.signatures?.length)
      fail("The signed trustline envelope did not match the approved operation.");
    status("Submitting signed Circle USDC trustline to Stellar Testnet…");
    const receipt=await horizon.submitTransaction(signed);
    if(!/^[a-f0-9]{64}$/.test(receipt.hash||""))fail("No valid Stellar transaction receipt.");
    showReceipt("https://stellar.expert/explorer/testnet/tx/"+receipt.hash,receipt.hash,"Circle Testnet USDC trustline");
    status("Circle Testnet USDC trustline confirmed: "+receipt.hash,"success");
  }finally{pending=false;}
}

function wireUi(){
  const supply=byId("review-availability");
  if(supply){
    supply.addEventListener("click",async event=>{
      if(!registry?.pilot)return;
      event.preventDefault();event.stopImmediatePropagation();
      const amount=byId("supply-amount")?.value;
      const asset=byId("supply-asset")?.value;
      if(asset!=="XLM"){status("Only native XLM is supported in the initial signed Testnet vault.");return;}
      const action=document.querySelector("[data-supply-tab][aria-selected='true']")?.dataset.supplyTab;
      try{
        const method=action==="withdraw"?"withdraw":"deposit";
        await signAction("vault",method,amount);
      }catch(error){status(error.message||"Transaction failed","error");}
    },true);
  }
  const borrow=byId("borrow-review");
  if(borrow){
    borrow.addEventListener("click",async event=>{
      if(!registry?.cfg?.activation?.marketOperationsEnabled)return;
      event.preventDefault();event.stopImmediatePropagation();
      const amount=byId("borrow-amount")?.value;
      const method=byId("borrow-action")?.value==="Repay"?"repay_for":"borrow";
      try{await signAction("market",method,amount);}
      catch(error){status(error.message||"Borrow transaction rejected","error");}
    },true);
  }

  byId("hoxo-oracle-activate")?.addEventListener("click",async()=>{
    try{await ownerControl("oracle");}catch(error){status(error.message||"Oracle activation error","error");}
  });
  byId("hoxo-market-activate")?.addEventListener("click",async()=>{
    try{await ownerControl("market");}catch(error){status(error.message||"Market activation error","error");}
  });
  for(const asset of ["XLM","USDC"]){
    byId("hoxo-price-"+asset.toLowerCase())?.addEventListener("click",async()=>{
      try{await submitPrice(asset);}catch(error){status(error.message||"Oracle quote rejected","error");}
    });
  }
  byId("hoxo-usdc-trustline")?.addEventListener("click",async()=>{
    try{await addUsdcTrustline();}catch(error){status(error.message||"USDC trustline failed","error");}
  });
  byId("hoxo-supply-market")?.addEventListener("click",async()=>{
    try{await signAction("market","supply",byId("market-supply-amount")?.value);}
    catch(error){status(error.message||"Market supply rejected","error");}
  });
  byId("hoxo-withdraw-market")?.addEventListener("click",async()=>{
    try{await signAction("market","withdraw_supply",byId("market-supply-amount")?.value);}
    catch(error){status(error.message||"Market withdrawal rejected","error");}
  });
  for(const method of ["deposit_collateral","withdraw_collateral"]){
    byId("hoxo-"+method.replaceAll("_","-"))?.addEventListener("click",async()=>{
      try{await signAction("market",method,byId("collateral-amount")?.value);}
      catch(error){status(error.message||"Collateral operation failed","error");}
    });
  }
  byId("hoxo-vault-activate")?.addEventListener("click",async()=>{
    try{await activateVault();}
    catch(error){status(error.message||"Activation failed","error");}
  });
  byId("hoxo-refresh-state")?.addEventListener("click",async()=>{
    try{await readRegistry();await refreshButtons();}
    catch(error){status(error.message||"RPC verification failed","error");}
  });
  document.addEventListener("click",event=>{
    if(event.target?.id==="connect-wallet"){
      setTimeout(()=>{void refreshButtons().catch(()=>{});},300);
    }
  });
}
async function start(){
  wireUi();
  try{await readRegistry();await refreshButtons();}
  catch(error){status(error.message||"Testnet readiness unknown","error");}
}
window.HoxoTradingBridge=Object.freeze({
  readWallet,readRegistry,readPaused,refreshButtons,signAction,activateVault,ownerControl,submitPrice,addUsdcTrustline
});
if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",start);
else void start();
