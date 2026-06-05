#!/usr/bin/env bash
# PostToolUse formatter. Reads the hook event JSON from stdin and formats the
# edited file by extension. Safe no-op if the toolchain or config is absent.
# Always exits 0 (PostToolUse cannot block, and a non-zero exit would only spam
# Claude with stderr). Requires python3 to parse the stdin JSON (present on macOS
# with Xcode CLT); if it is missing, file_path is empty and the hook no-ops.

file_path="$(python3 -c 'import sys, json; print(json.load(sys.stdin).get("tool_input", {}).get("file_path", ""))' 2>/dev/null || true)"

[ -z "$file_path" ] && exit 0
[ -f "$file_path" ] || exit 0

case "$file_path" in
  *.ts|*.tsx|*.js|*.jsx|*.json|*.css|*.html)
    npx --no-install prettier --write "$file_path" >/dev/null 2>&1 || true
    ;;
  *.rs)
    rustfmt --edition 2021 "$file_path" >/dev/null 2>&1 || true
    ;;
esac

exit 0
