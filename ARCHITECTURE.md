# ARCHITECTURE.md

> Architecture of the scaffolded Tauri 2.x app (React 19 + TypeScript 6 frontend, Rust backend).

## Overview

A Tauri 2.x desktop app made of two processes:

- **Frontend** (`src/`): a TypeScript web UI rendered in Tauri's webview. Owns presentation and user interaction.
- **Backend** (`src-tauri/`): a Rust binary hosting the webview, OS integration, and privileged operations (filesystem, native APIs).

They communicate over Tauri's IPC bridge.

## IPC and commands

- Rust functions exposed to the frontend are `#[tauri::command]` handlers registered in `src-tauri/src/lib.rs`.
- Each command must be granted in a capability file under `src-tauri/capabilities/` before the frontend may call it. This is the security boundary: the frontend can only invoke explicitly allowed commands.
- The frontend calls commands via `@tauri-apps/api`'s `invoke()`.

## Configuration

- `src-tauri/tauri.conf.json` — app identifier, window config, `devUrl`, `frontendDist`, `beforeDevCommand` / `beforeBuildCommand` (wire the frontend bundler into the Tauri CLI lifecycle), and bundle settings.

## Build

- `npm run tauri dev` compiles the Rust backend, starts the frontend dev server, and opens the window with hot-reload.
- `npm run tauri build` produces a platform-native installer/bundle.

## Boundaries to respect

- Validate external input (IPC payloads, file contents) at the Rust command boundary.
- Keep `main.rs` a thin shim; put logic in `lib.rs` so the mobile entry point stays shared.
