import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const pageFiles = [
  "index.html","app/index.html","app/markets/index.html",
  "app/markets/xlm/index.html","app/markets/usdc/index.html",
  "app/supply/index.html","app/borrow/index.html","app/farm/index.html",
  "app/credit/index.html","app/portfolio/index.html","app/analytics/index.html",
  "app/protocol/index.html","app/risk/index.html","app/transactions/index.html",
  "app/settings/index.html","app/faucet/index.html","app/liquidations/index.html",
  "app/rwa/index.html","app/lab/index.html","app/readiness/index.html","docs/index.html"
];
const routes = new Set([
  "/","/app/","/app/markets/","/app/markets/xlm/","/app/markets/usdc/",
  "/app/supply/","/app/borrow/","/app/farm/","/app/credit/","/app/portfolio/",
  "/app/analytics/","/app/protocol/","/app/risk/","/app/transactions/",
  "/app/settings/","/app/faucet/","/app/liquidations/","/app/rwa/",
  "/app/lab/","/app/readiness/","/docs/"
]);
assert.ok(existsSync("src/v2.css"));
assert.ok(existsSync("src/v2.js"));
const css = readFileSync("src/v2.css","utf8");
const js = readFileSync("src/v2.js","utf8");
assert.match(css,/#1037ff/i);
assert.match(css,/#fff(?:fff)?\b/i);
assert.match(js,/requestAccess/);
assert.match(js,/networkPassphrase/);
assert.doesNotMatch(js,/signTransaction\s*\(|submitTransaction\s*\(/);

const cfg = JSON.parse(readFileSync("config/stellar-testnet.json","utf8"));
assert.equal(cfg.network,"Stellar Testnet");
assert.equal(cfg.deploymentStatus,"deployed-paused");
assert.equal(cfg.lendingEnabled,false);
assert.equal(cfg.tradingEnabled,false);
assert.equal(cfg.deploymentEvidence?.ephemeralAdmin,true);

for(const file of pageFiles) {
  const html = readFileSync(file,"utf8");
  assert.match(html,/<html lang="en">/,file);
  assert.match(html,/<title>/,file);
  assert.match(html,/\/src\/v2\.css/,file);
  assert.match(html,/\/src\/v2\.js/,file);
  assert.match(html,/Stellar Testnet/i,file);
  assert.match(html,/paused/i,file);
  assert.doesNotMatch(html,/bitkub|WtKUB|KUB Testnet|KUBChain|Maddeth|25925|0x[a-f0-9]{40}/i,file);
  assert.ok(!/>\s*\$[0-9][\d,.]*[MK]?(?:\s|<)/.test(html),file+" must not invent TVL");
  const links=[...html.matchAll(/<a\b[^>]*href="(\/[^"#?]*)"/gi)].map(m=>m[1]);
  for(const href of links) {
    const normalized = href === "/" ? href : href.endsWith("/") ? href : href + "/";
    assert.ok(routes.has(normalized),file+": broken local navigation link "+href);
  }
}
const nav = readFileSync("app/index.html","utf8");
for(const item of ["Dashboard","Markets","Supply","Borrow","Farm","Credit",
                   "Portfolio","Analytics","Protocol","Docs"]) assert.ok(nav.includes(item),item);
const home=readFileSync("index.html","utf8");
assert.match(home,/Capital/);
assert.match(home,/Without Limits/);
assert.match(home,/Native Credit Markets on Stellar/);
assert.match(readFileSync("app/supply/index.html","utf8"),/review-availability/);
assert.match(readFileSync("app/borrow/index.html","utf8"),/borrow-preview/);
assert.match(readFileSync("app/protocol/index.html","utf8"),/contract-list/);
assert.match(readFileSync("app/analytics/index.html","utf8"),/data-period/);
console.log("Hoxo Lend V2: Stellar-only routes, branded design, wallet guards and no-invented-data checks passed (" + pageFiles.length + " pages).");