# Repository Guidelines

## Shared Agent Instructions

`CLAUDE.md` is a symlink to this file. Treat `AGENTS.md` as the canonical source for contributor and agent instructions: update `AGENTS.md` and keep the symlink intact so Claude receives the same injected context.

## Project Structure & Module Organization

This repository builds an x402-enabled vending terminal. `backend/` contains the TypeScript/Express service: application code lives in `backend/src/`, route handlers in `backend/src/routes/`, and Vitest/Supertest integration tests in `backend/test/`. `firmware/` contains the Rust ESP32-S3 application; keep hardware concerns separated across `src/api.rs`, `display.rs`, `touch.rs`, `vend.rs`, and `wifi.rs`. The system design and payment flow are documented in `docs/superpowers/specs/`. Update `bom.md` when hardware selections change.

## Build, Test, and Development Commands

Run backend commands from `backend/`:

- `npm ci` installs the exact dependencies recorded in `package-lock.json`.
- `npm run dev` starts the API with automatic TypeScript reloads.
- `npm run build` compiles `src/` into `dist/`.
- `npm test` runs the Vitest suite once; `npm run test:watch` supports iteration.
- `npm start` runs the compiled server.

Run firmware commands from `firmware/` after installing the esp-rs `esp` toolchain (`espup install`, plus `probe-rs`, `espflash`, `ldproxy`, and Python on PATH):

- `cargo build` compiles the ESP-IDF firmware.
- `cargo fmt --check` verifies Rust formatting; `cargo fmt` applies it.
- `cargo check` provides a faster compile-time validation pass.
- `cargo run` flashes over the ESP32-S3's built-in USB-JTAG via probe-rs; watch logs with `espflash monitor` (USB-Serial-JTAG console).

On Windows, before building, source `%USERPROFILE%\export-esp.ps1` and set `ESP_IDF_TOOLS_INSTALL_DIR=global` plus a short `CARGO_TARGET_DIR` (e.g. `C:\esp\t`) — the ESP-IDF CMake build fails with "Too long output directory" under a normal project path.

## Coding Style & Naming Conventions

TypeScript uses strict mode, ES modules, two-space indentation, double quotes, and semicolons. Use `camelCase` for values and functions, `PascalCase` for types and classes, and kebab-case filenames such as `session-store.ts`. Rust follows `rustfmt`: four-space indentation, `snake_case` modules/functions, and `PascalCase` types and enum variants. Keep pure logic direct; reserve error handling for network, disk, device, and untrusted-input boundaries.

## Testing Guidelines

Add backend tests as `backend/test/<feature>.test.ts`. Group them by endpoint with `describe`, and name cases by observable behavior. Cover success, invalid input, missing resources, and facilitator failures. Mock external payment calls; never settle live funds in tests. Firmware has no automated suite yet, so run formatting and compile checks and describe any on-device verification in the pull request.

## Commit & Pull Request Guidelines

Use short, imperative subjects. Follow existing prefixes where appropriate: `feat:`, `fix:`, or `scaffold:`. Keep commits focused on one concern. Pull requests should explain behavior changes, link relevant issues or design sections, list commands run, and call out configuration or hardware effects. Include terminal logs or UI photos when firmware, display, touch, or relay behavior changes. Never commit `.env`, credentials, wallet keys, or production payment signatures.
