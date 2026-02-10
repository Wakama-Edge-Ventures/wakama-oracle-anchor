#!/usr/bin/env bash
set -euo pipefail

ANCHOR_REPO="$HOME/dev/wakama/wakama-oracle-anchor"
DASHBOARD_REPO="$HOME/dev/wakama/wakama-dashboard"

PROOFS_DIR="$ANCHOR_REPO/proofs/mainnet/wakama-capital-pool"
EXPORT_DIR="$ANCHOR_REPO/export/mainnet/wakama-capital-pool"
DASHBOARD_PUBLIC_DIR="$DASHBOARD_REPO/public/capital-pool/mainnet"

cd "$ANCHOR_REPO"

echo "==> 1) Generate proofs (summary.json + PROOF.md)"
node scripts/prove-by-team.js \
  --receipts "receipts/mainnet/wakama-capital-pool" \
  --out "proofs/mainnet/wakama-capital-pool"

echo "==> 2) Build receipts index (receipts.index.json)"
node - <<'NODE'
const fs = require("fs");
const path = require("path");

const root = "receipts/mainnet/wakama-capital-pool";
function readJson(p){ return JSON.parse(fs.readFileSync(p,"utf8")); }

const items = [];
for (const teamId of fs.readdirSync(root)) {
  const dir = path.join(root, teamId);
  if (!fs.statSync(dir).isDirectory()) continue;

  for (const f of fs.readdirSync(dir).filter(x => x.endsWith(".json"))) {
    const p = path.join(dir, f);
    const j = readJson(p);

    items.push({
      teamId: j.teamId ?? teamId,
      createdAt: j.createdAt ?? null,
      amountUsdc: j.amountUsdc ?? null,
      amountBaseUnits: j.amountBaseUnits ?? null,
      tx: j.tx ?? null,
      receiptFile: f,
      receiptPath: p,
    });
  }
}
items.sort((a,b)=> (a.createdAt > b.createdAt ? -1 : 1));

fs.mkdirSync("export/mainnet/wakama-capital-pool", { recursive: true });
fs.writeFileSync(
  "export/mainnet/wakama-capital-pool/receipts.index.json",
  JSON.stringify({ generatedAt: new Date().toISOString(), count: items.length, items }, null, 2)
);

console.log("OK: export/mainnet/wakama-capital-pool/receipts.index.json");
NODE

echo "==> 3) Copy proof files to export/"
mkdir -p "$EXPORT_DIR"
cp "$PROOFS_DIR/summary.json" "$EXPORT_DIR/summary.json"
cp "$PROOFS_DIR/PROOF.md" "$EXPORT_DIR/PROOF.md"

echo "==> 4) Publish to dashboard public folder (rsync)"
mkdir -p "$DASHBOARD_PUBLIC_DIR"
rsync -av --delete "$EXPORT_DIR/" "$DASHBOARD_PUBLIC_DIR/"

echo "==> DONE"
echo "Published files:"
echo " - $DASHBOARD_PUBLIC_DIR/summary.json"
echo " - $DASHBOARD_PUBLIC_DIR/PROOF.md"
echo " - $DASHBOARD_PUBLIC_DIR/receipts.index.json"
echo
echo "URLs (after deploy):"
echo " - https://rwa.wakama.farm/capital-pool/mainnet/summary.json"
echo " - https://rwa.wakama.farm/capital-pool/mainnet/PROOF.md"
echo " - https://rwa.wakama.farm/capital-pool/mainnet/receipts.index.json"