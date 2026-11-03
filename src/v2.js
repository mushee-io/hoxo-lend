/* Hoxo Lend V2 — Stellar Testnet read-only UI controls and Freighter connection.
   There is NO transaction signing or submission on the deployed-paused protocol. */
(() => {
  "use strict";
  const NETWORK_PASSPHRASE = "Test SDF Network ; September 2015";
  const STELLAR_ACCOUNT = /^G[A-Z2-7]{55}$/;
  const CONTRACT_ID = /^C[A-Z2-7]{55}$/;
  const REQUIRED = ["vault","market","oracle","interestRateModel","rwaVaultFactory","rwaVault","lens"];
  const CONTRACT_LABELS = {
    market:"Isolated lending market",vault:"Custody vault",oracle:"Price oracle",
    interestRateModel:"Interest-rate model",rwaVaultFactory:"RWA vault factory",
    rwaVault:"RWA credit vault",lens:"Account-data lens"
  };
  const $$ = (selector, scope = document) => [...scope.querySelectorAll(selector)];
  const $ = (selector, scope = document) => scope.querySelector(selector);
  const safeText = (selector, content) => { $$(selector).forEach(node => { node.textContent = content; }); };
  let config = null;
  let wallet = null;
  let currentAction = "deposit";
  let selectedPeriod = "7D";

  const toast = (text) => {
    let box = $("#hoxo-toast");
    if (!box) {
      box = document.createElement("div");
      box.id = "hoxo-toast";
      box.className = "toast";
      box.setAttribute("role", "status");
      document.body.appendChild(box);
    }
    box.hidden = false;
    box.textContent = text;
    clearTimeout(box._hide);
    box._hide = setTimeout(() => { box.hidden = true; }, 6500);
  };
  const isVerified = (c) =>
    Boolean(c && c.network === "Stellar Testnet" && c.passphrase === NETWORK_PASSPHRASE &&
      c.deploymentStatus === "deployed-paused" && c.lendingEnabled === false &&
      c.deploymentEvidence?.allContractsPaused === true &&
      REQUIRED.every(key => CONTRACT_ID.test(c.contracts?.[key] || "")));

  async function loadConfig() {
    try {
      const response = await fetch("/config/stellar-testnet.json", { cache:"no-store" });
      if (!response.ok) throw new Error("Network registry could not be loaded");
      const candidate = await response.json();
      if (candidate.network !== "Stellar Testnet" || candidate.passphrase !== NETWORK_PASSPHRASE ||
          candidate.lendingEnabled !== false || candidate.tradingEnabled !== false) {
        throw new Error("Unexpected network configuration. All transactions disabled.");
      }
      config = candidate;
      renderNetworkState();
      renderContracts();
      renderMetrics();
    } catch (error) {
      safeText("[data-network-status]", "Registry unavailable");
      safeText("[data-verification-status]", "Unverified — cannot display contract details");
      toast(error.message || "Could not validate the Testnet registry");
    }
  }

  function renderNetworkState() {
    const verified = isVerified(config);
    safeText("[data-network-status]", verified ? "STELLAR TESTNET / DEPLOYED & PAUSED" : "STELLAR TESTNET / UNVERIFIED");
    safeText("[data-verification-status]", verified ? "Seven Soroban contract IDs verified in deployment registry" : "Verification pending");
    safeText("[data-deployment-count]", verified ? "07" : "—");
    safeText("[data-release-status]", verified ? "Contracts deployed · Operations paused" : "Deployment verification unavailable");
  }

  function renderMetrics() {
    if (!config) return;
    const active = Array.isArray(config.plannedMarkets)
      ? config.plannedMarkets.filter(market => market.active && CONTRACT_ID.test(market.marketContractId || "")).length
      : 0;
    safeText('[data-stat="active-markets"]', String(active));
    safeText('[data-stat="contracts"]', isVerified(config) ? "7" : "—");
    // Values that depend on an audited indexer must never default to fake zero.
    for (const field of ["total-supplied","total-borrowed","available-liquidity",
                         "utilization","supply-apy","borrow-apr","reserves","accounts"]) {
      safeText('[data-stat="' + field + '"]', "—");
    }
  }

  function createExplorerLink(id) {
    const a = document.createElement("a");
    a.href = "https://stellar.expert/explorer/testnet/contract/" + id;
    a.rel = "noopener noreferrer";
    a.target = "_blank";
    a.className = "address";
    a.textContent = id.slice(0, 9) + "…" + id.slice(-8) + " ↗";
    a.title = id;
    return a;
  }

  function renderContracts() {
    const list = $("#contract-list");
    if (!list || !isVerified(config)) return;
    list.replaceChildren();
    for (const key of REQUIRED) {
      const li = document.createElement("li");
      const label = document.createElement("span");
      label.textContent = CONTRACT_LABELS[key];
      li.appendChild(label);
      li.appendChild(createExplorerLink(config.contracts[key]));
      list.appendChild(li);
    }
    const proof = $("#deployment-proof-link");
    const url = config.deploymentEvidence?.runUrl;
    if (proof && /^https:\/\/github\.com\/mushee-io\/hoxo-lend\/actions\/runs\/\d+$/.test(url || "")) {
      proof.href = url;
      proof.hidden = false;
    }
  }

  function setupMobileNavigation() {
    const siteToggle = $("#site-mobile-toggle");
    const siteNav = $("#site-nav");
    siteToggle?.addEventListener("click", () => {
      const open = siteNav.classList.toggle("open");
      siteToggle.setAttribute("aria-expanded", String(open));
    });
    const mobileButton = $("#app-mobile-toggle");
    const rail = $("#app-sidebar");
    const backdrop = $("#mobile-backdrop");
    function hideRail() {
      rail?.classList.remove("open");
      backdrop?.classList.remove("open");
      mobileButton?.setAttribute("aria-expanded", "false");
    }
    mobileButton?.addEventListener("click", () => {
      const visible = rail?.classList.toggle("open");
      backdrop?.classList.toggle("open", Boolean(visible));
      mobileButton.setAttribute("aria-expanded", String(Boolean(visible)));
    });
    backdrop?.addEventListener("click", hideRail);
    window.addEventListener("keydown", event => { if (event.key === "Escape") hideRail(); });
  }

  async function checkNetwork() {
    const status = $("#rpc-health");
    const button = $("#check-rpc");
    if (!status || !button) return;
    status.textContent = "Checking Stellar Testnet RPC…";
    button.disabled = true;
    try {
      if (!config) throw new Error("Registry not loaded");
      const response = await fetch(config.rpcUrl, {
        method:"POST",
        headers: { "Content-Type":"application/json" },
        body:JSON.stringify({jsonrpc:"2.0",id:1,method:"getHealth"})
      });
      if (!response.ok) throw new Error("RPC HTTP " + response.status);
      const result = await response.json();
      if (result.error) throw new Error(result.error.message || "RPC failed");
      status.textContent = result.result?.status === "healthy"
        ? "RPC healthy · Testnet · lending paused"
        : "RPC responded, health not confirmed";
    } catch (error) {
      status.textContent = "RPC check unavailable: " + (error.message || "unknown error");
    } finally { button.disabled = false; }
  }

  // Freighter version 6.0.1 is loaded as a separately pinned script on app pages.
  // The protocol is paused; this connect flow ONLY requests access to a public key.
  async function connectWallet() {
    const button = $("#connect-wallet");
    if (!button) return;
    if (wallet) {
      wallet = null;
      safeText("[data-wallet-address]", "Not connected");
      button.textContent = "Connect Wallet";
      toast("Wallet disconnected from this page. Freighter's authorization can be managed inside the extension.");
      return;
    }
    const api = window.freighterApi;
    if (!api || typeof api.isConnected !== "function") {
      toast("Freighter API unavailable. Install Freighter on desktop or check that the extension is enabled.");
      return;
    }
    button.disabled = true;
    button.textContent = "Connecting…";
    try {
      const available = await api.isConnected();
      if (available.error || !available.isConnected) throw new Error("Freighter extension not detected");
      const access = await api.requestAccess();
      if (access.error || !STELLAR_ACCOUNT.test(access.address || "")) {
        throw new Error(access.error?.message || "A valid Stellar account was not returned");
      }
      const network = await api.getNetwork();
      if (network.error || network.networkPassphrase !== NETWORK_PASSPHRASE) {
        throw new Error("Switch Freighter to Stellar Testnet before connecting");
      }
      wallet = { address: access.address };
      safeText("[data-wallet-address]", access.address.slice(0,6) + "…" + access.address.slice(-5));
      button.textContent = access.address.slice(0, 5) + "…" + access.address.slice(-4) + " · Disconnect";
      toast("Connected on Stellar Testnet (read-only). Lending transactions remain disabled.");
    } catch (error) {
      toast(error.message || "Could not connect Freighter");
      button.textContent = "Connect Wallet";
    } finally {
      button.disabled = false;
    }
  }

  function setupMarketTable() {
    const search = $("#market-search");
    const sort = $("#market-sort");
    const filter = $("#market-filter");
    const rows = $$("[data-market-row]");
    if (!rows.length) return;
    const empty = $("#market-empty");
    function refresh() {
      const term = (search?.value || "").trim().toLowerCase();
      const mode = filter?.value || "all";
      const matching = rows.filter(row => {
        const symbol = (row.dataset.symbol || "").toLowerCase();
        const listed = row.dataset.listed === "true";
        return (mode === "all" || (mode === "active" && listed) || (mode === "planned" && !listed)) &&
          (!term || symbol.includes(term) || row.textContent.toLowerCase().includes(term));
      });
      for (const row of rows) row.hidden = !matching.includes(row);
      if (sort) {
        const tbody = rows[0]?.parentElement;
        const sorted = [...rows].sort((a,b) =>
          (a.dataset.symbol || "").localeCompare(b.dataset.symbol || "") * (sort.value === "reverse" ? -1 : 1));
        sorted.forEach(row => tbody.appendChild(row));
      }
      if (empty) empty.hidden = matching.length > 0;
    }
    search?.addEventListener("input", refresh);
    sort?.addEventListener("change", refresh);
    filter?.addEventListener("change", refresh);
    refresh();
  }

  function setupSupplyWidget() {
    const amount = $("#supply-amount");
    const asset = $("#supply-asset");
    const preview = $("#supply-preview");
    const max = $("#supply-max");
    const button = $("#review-availability");
    if (!amount || !asset || !preview) return;
    const tabs = $$("[data-supply-tab]");
    function selectAction(action) {
      currentAction = action;
      tabs.forEach(tab => tab.setAttribute("aria-selected", String(tab.dataset.supplyTab === action)));
      safeText("[data-action-name]", action === "deposit" ? "Deposit" : "Withdraw");
      amount.value = "";
      preview.textContent = "Enter an amount to inspect availability. This is not a transaction simulation.";
    }
    tabs.forEach(tab => tab.addEventListener("click", () => selectAction(tab.dataset.supplyTab)));
    max?.addEventListener("click", () => toast("MAX requires verified wallet balances and a live approved lending market."));
    button?.addEventListener("click", () => {
      const input = amount.value.trim();
      const decimal = Number(input);
      if (!input || !Number.isFinite(decimal) || decimal <= 0 || !/^\d+(?:\.\d{1,7})?$/.test(input)) {
        preview.textContent = "Enter a positive amount with at most seven decimal places.";
        return;
      }
      preview.textContent =
        currentAction.toUpperCase() + " " + input + " " + asset.value +
        ": unavailable. The market is paused, asset approval and wallet-signed transaction preflight are incomplete. No transaction was submitted.";
    });
  }

  function setupBorrowWidget() {
    const amount = $("#borrow-amount");
    const action = $("#borrow-action");
    const review = $("#borrow-review");
    const output = $("#borrow-preview");
    if (!amount || !review) return;
    review.addEventListener("click", () => {
      const value = amount.value.trim();
      if (!/^\d+(?:\.\d{1,7})?$/.test(value) || Number(value) <= 0) {
        output.textContent = "Enter a positive amount with no more than seven decimal places.";
        return;
      }
      output.textContent = (action?.value || "Borrow") + " " + value +
        " XLM: preview only. Live debt, collateral prices and borrowing capacity cannot be obtained from a paused market. No wallet request has been issued.";
    });
  }

  function setupChartRanges() {
    const buttons = $$("[data-period]");
    for (const button of buttons) {
      button.addEventListener("click", () => {
        selectedPeriod = button.dataset.period;
        buttons.forEach(item => item.setAttribute("aria-pressed", String(item === button)));
        safeText("[data-chart-range]", selectedPeriod);
        safeText("[data-chart-message]", "No indexed Hoxo Lend lending history available for " + selectedPeriod + ".");
      });
    }
  }

  function exportEmptyCSV() {
    const rows = [["network","date","market","action","amount","transaction_hash"]];
    const csv = rows.map(row => row.join(",")).join("\n") + "\n";
    const blob = new Blob([csv], { type:"text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "hoxo-lend-testnet-activity-empty.csv";
    a.click();
    URL.revokeObjectURL(url);
    toast("Exported column headers only. There is no independently indexed transaction history yet.");
  }

  function setupUtilities() {
    $("#check-rpc")?.addEventListener("click", checkNetwork);
    $("#connect-wallet")?.addEventListener("click", connectWallet);
    $("#export-csv")?.addEventListener("click", exportEmptyCSV);
    $$("[data-open-panel]").forEach(trigger => {
      trigger.addEventListener("click", () => {
        const panel = $("#" + trigger.dataset.openPanel);
        if (!panel) return;
        const opening = panel.getAttribute("aria-hidden") !== "false";
        $$("[data-utility-panel]").forEach(p => { p.setAttribute("aria-hidden","true"); });
        panel.setAttribute("aria-hidden", String(!opening));
      });
    });
    $$("[data-copy-contract]").forEach(button => button.addEventListener("click", async () => {
      const key = button.dataset.copyContract;
      const address = config?.contracts?.[key];
      if (!CONTRACT_ID.test(address || "")) { toast("No verified contract address available."); return; }
      try { await navigator.clipboard.writeText(address); toast("Copied verified contract address."); }
      catch { toast("Clipboard unavailable. Open the contract from the explorer list."); }
    }));
  }

  function boot() {
    setupMobileNavigation();
    setupMarketTable();
    setupSupplyWidget();
    setupBorrowWidget();
    setupChartRanges();
    setupUtilities();
    void loadConfig();
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", boot);
  else boot();
})();