#!/usr/bin/env bash
# Re-stamp this scaffold for a new project.
#
# Usage: scripts/rename.sh <app-name> [bundle-id] [product-name]
#   <app-name>      kebab-case, e.g. my-app
#   [bundle-id]     reverse-DNS, default: com.hleb.<app-name without hyphens>
#   [product-name]  display/window name, default: <app-name>
#
# Replaces the resting scaffold identity:
#   package/crate  tauri-starter / tauri_starter_lib
#   bundle id      com.hleb.starter
#   product/title  "Tauri Starter"
set -euo pipefail

NAME="${1:?usage: scripts/rename.sh <app-name> [bundle-id] [product-name]}"
LIB="${NAME//-/_}_lib"
ID="${2:-com.hleb.${NAME//-/}}"
PRODUCT="${3:-$NAME}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

repl() { perl -0pi -e "s{\Q$1\E}{$2}g" "$3"; }

repl "tauri-starter"     "$NAME"    package.json
repl "tauri-starter"     "$NAME"    src-tauri/Cargo.toml
repl "tauri_starter_lib" "$LIB"     src-tauri/Cargo.toml
repl "tauri_starter_lib" "$LIB"     src-tauri/src/main.rs
repl "Tauri Starter"     "$PRODUCT" src-tauri/tauri.conf.json
repl "com.hleb.starter"  "$ID"      src-tauri/tauri.conf.json

echo "Renamed: name=$NAME  lib=$LIB  id=$ID  product=\"$PRODUCT\""
echo "Next:"
echo "  rm -rf node_modules dist src-tauri/target src-tauri/Cargo.lock"
echo "  npm install && cargo check --manifest-path src-tauri/Cargo.toml"
echo "  git config core.hooksPath .githooks"
