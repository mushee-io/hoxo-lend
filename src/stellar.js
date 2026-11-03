// Hoxo Lend Testnet-only public deployment registry and RPC health.
// Read-only. Never creates, signs, simulates, or submits a financial transaction.
const CONTRACT_ID = /^C[A-Z2-7]{55}$/;
const VERIFIED_STATES = new Set(["not-deployed", "deployed-paused"]);
let registryCache = null;

async function readRegistry() {
  const response = await fetch("/config/stellar-testnet.json", { cache: "no-store" });
  if (!response.ok) throw new Error("Testnet registry unavailable");
  const cfg = await response.json();
  if (cfg.network !== "Stellar Testnet" ||
      cfg.passphrase !== "Test SDF Network ; September 2015" ||
      !VERIFIED_STATES.has(cfg.deploymentStatus) ||
      cfg.lendingEnabled !== false || cfg.tradingEnabled !== false) {
    throw new Error("Unexpected network or unsafe lending configuration");
  }
  return cfg;
}

function readOnlyDeployed(cfg) {
  const expected = ["vault", "oracle", "interestRateModel", "rwaVaultFactory",
                    "rwaVault", "lens", "market"];
  return cfg.deploymentStatus === "deployed-paused" &&
    cfg.deploymentEvidence?.allContractsPaused === true &&
    expected.every(key => CONTRACT_ID.test(cfg.contracts?.[key] || ""));
}
function addText(tag, value, parent, className) {
  const el = document.createElement(tag);
  if (className) el.className = className;
  el.textContent = value;
  parent.appendChild(el);
  return el;
}
function explorerLink(contractId, label, parent) {
  if (!CONTRACT_ID.test(contractId)) return;
  const a = document.createElement("a");
  a.href = "https://stellar.expert/explorer/testnet/contract/" + contractId;
  a.target = "_blank";
  a.rel = "noopener noreferrer";
  a.textContent = label + ": " + contractId;
  a.style.overflowWrap = "anywhere";
  parent.appendChild(a);
}
function showReadOnlyDeployment(cfg) {
  if (!readOnlyDeployed(cfg)) return;
  const headline = "STELLAR TESTNET · 7 CONTRACTS DEPLOYED · ALL PAUSED · NO PUBLIC LENDING";
  const topline = document.querySelector(".topline");
  if (topline) topline.textContent = headline;
  const warning = document.querySelector(".preview-warning");
  if (warning) {
    warning.replaceChildren();
    addText("strong", "Real Stellar Testnet deployment — paused. ", warning);
    warning.appendChild(document.createTextNode(
      "The seven Soroban contracts have recorded Testnet IDs, but borrowing, supply and live positions are not activated. No public funds are accepted."
    ));
  }
  const homeStatus = document.querySelector(".status");
  if (homeStatus) {
    homeStatus.replaceChildren();
    addText("span", "●", homeStatus);
    addText("strong", "Stellar Testnet deployed, paused", homeStatus);
    addText("span", "No live lending or public deposits", homeStatus);
  }
  const homeNote = document.querySelector(".note");
  if (homeNote) {
    homeNote.replaceChildren();
    addText("strong", "Current launch state: deployed to Testnet, paused. ", homeNote);
    homeNote.appendChild(document.createTextNode(
      "All seven Soroban contract IDs are registered below. Independent audit, persistent administrator control, Freighter transaction signing and full financial acceptance remain outstanding."
    ));
  }

  const host = document.querySelector("main .shell") || document.querySelector("main");
  if (!host || document.getElementById("hoxo-deployment-evidence")) return;
  const section = document.createElement("section");
  section.id = "hoxo-deployment-evidence";
  section.className = "panel";
  section.style.margin = "24px auto 70px";
  section.style.maxWidth = "1220px";
  addText("div", "ON-CHAIN EVIDENCE · STELLAR TESTNET", section, "eyebrow");
  addText("h2", "Deployed Soroban contracts", section);
  addText("p", "Read-only verified addresses. Contract execution is paused; live balances and lending APYs are not available.", section);
  const names = {
    market: "Lending market", vault: "Custody vault", oracle: "Price oracle",
    interestRateModel: "Interest-rate model", rwaVault: "RWA vault",
    rwaVaultFactory: "RWA factory", lens: "Read-only market lens"
  };
  const list = document.createElement("ul");
  for (const [key, title] of Object.entries(names)) {
    const item = document.createElement("li");
    explorerLink(cfg.contracts[key], title, item);
    list.appendChild(item);
  }
  section.appendChild(list);
  const ev = cfg.deploymentEvidence || {};
  if (ev.ephemeralAdmin) {
    addText("p", "Administrative key: ephemeral deployment identity. These contracts cannot be reopened by the public; a team-controlled administrator deployment is required.", section);
  }
  if (typeof ev.runUrl === "string" &&
      /^https:\/\/github\.com\/mushee-io\/hoxo-lend\/actions\/runs\/\d+$/.test(ev.runUrl)) {
    const a = document.createElement("a");
    a.href = ev.runUrl;
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    a.textContent = "Review deployment workflow and verification";
    section.appendChild(a);
  }
  host.appendChild(section);
}

async function initializeStatus() {
  try {
    registryCache = await readRegistry();
    showReadOnlyDeployment(registryCache);
  } catch (err) {
    const health = document.getElementById("network-health");
    if (health) health.textContent = "Registry unavailable: " + (err.message || "unknown error");
  }
}
async function checkNetwork() {
  const label = document.getElementById("network-health");
  const button = document.getElementById("check-network");
  if (!label || !button) return;
  button.disabled = true;
  label.textContent = "Checking Stellar Testnet RPC…";
  try {
    const cfg = registryCache || await readRegistry();
    const rpcResponse = await fetch(cfg.rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({jsonrpc:"2.0",id:1,method:"getHealth"})
    });
    if (!rpcResponse.ok) throw new Error("RPC HTTP " + rpcResponse.status);
    const data = await rpcResponse.json();
    if (data.error) throw new Error(data.error.message || "RPC error");
    if (data.result?.status !== "healthy") throw new Error("RPC not healthy");
    label.textContent = readOnlyDeployed(cfg)
      ? "Stellar Testnet RPC healthy · Hoxo Lend deployed, paused · lending not enabled"
      : "Stellar Testnet RPC healthy · Hoxo Lend contracts not deployed";
  } catch (error) {
    label.textContent = "RPC status unavailable · " + (error.message || "failed");
  } finally {
    button.disabled = false;
  }
}
document.getElementById("check-network")?.addEventListener("click", checkNetwork);
void initializeStatus();
