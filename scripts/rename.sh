#!/usr/bin/env bash
# Re-stamp this scaffold for a new project: swaps the package/crate/bundle-id/
# product name across all files that carry it, removes scaffold-only doc
# sections, drops the lockfiles (regenerated on install), then deletes itself.
# Run once, on a copy.
#
# Usage: scripts/rename.sh <app-name> [bundle-id] [product-name]
#   <app-name>      kebab-case, e.g. my-app
#   [bundle-id]     reverse-DNS, default: com.hleb.<app-name without hyphens>
#   [product-name]  display/window name, default: <app-name>
set -euo pipefail

NAME="${1:?usage: scripts/rename.sh <app-name> [bundle-id] [product-name]}"
LIB="${NAME//-/_}_lib"
ID="${2:-com.hleb.${NAME//-/}}"
PRODUCT="${3:-$NAME}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

repl() { perl -0pi -e "s{\Q$1\E}{$2}g" "$3"; }

# Every file that carries the resting identity (code, frontend, docs, release).
FILES=(
  package.json
  index.html
  src/App.tsx
  src-tauri/Cargo.toml
  src-tauri/src/main.rs
  src-tauri/tauri.conf.json
  .github/workflows/release.yml
  README.md
  AGENTS.md
  CLAUDE.md
  ARCHITECTURE.md
)
for f in "${FILES[@]}"; do
  [ -f "$f" ] || continue
  repl "tauri_starter_lib" "$LIB" "$f"
  repl "tauri-starter" "$NAME" "$f"
  repl "com.hleb.starter" "$ID" "$f"
  repl "Tauri Starter" "$PRODUCT" "$f"
done

# Strip scaffold-only doc blocks (everything between the markers, inclusive).
strip_blocks() {
  [ -f "$1" ] || return 0
  perl -0pi -e 's/[ \t]*<!-- starter:remove-start -->.*?<!-- starter:remove-end -->\n?//gs' "$1"
}
for f in README.md AGENTS.md CLAUDE.md; do strip_blocks "$f"; done

# Lockfiles carry the old name; drop them so npm/cargo regenerate cleanly.
rm -f package-lock.json src-tauri/Cargo.lock

echo "Renamed: name=$NAME  lib=$LIB  id=$ID  product=\"$PRODUCT\""
echo "Now regenerate and install:"
echo "  rm -rf node_modules dist src-tauri/target"
echo "  npm install && cargo check --manifest-path src-tauri/Cargo.toml"
echo "  git config core.hooksPath .githooks"

# Restamping is one-time: remove this script so the new project is pristine.
echo "(removing scripts/rename.sh — restamp is one-time)"
rm -- "$0"
rmdir scripts 2>/dev/null || true
