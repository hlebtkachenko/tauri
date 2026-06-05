# Security Policy

## Reporting a vulnerability

Report security issues privately via GitHub's "Report a vulnerability" (repository Security tab, Advisories), not a public issue.

## Automated measures

- **Secret scanning**: gitleaks runs in CI on every push and pull request, and locally via a pre-commit hook (`.githooks/pre-commit`). Enable the hook once per clone with `git config core.hooksPath .githooks`.
- **Dependency updates**: Dependabot watches npm, Cargo, and GitHub Actions weekly.
- **CI**: type-check, build, and `cargo fmt`/`clippy`/`check`/`test` run on every push and pull request.

## Tauri hardening notes

- `src-tauri/tauri.conf.json` ships `app.security.csp: null` (scaffold default). Before release, set a restrictive Content Security Policy. A strict CSP can break the Vite dev server (HMR, inline assets), so tune it against `npm run tauri dev`.
- Only commands granted in `src-tauri/capabilities/` are reachable from the frontend. Keep that allowlist minimal.
- Never commit secrets; `.gitignore` and the pre-commit hook guard against `.env` files and keys.
