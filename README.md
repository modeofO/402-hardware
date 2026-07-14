# x402 Vending Terminal

An ESP32-S3 terminal that lets a vending machine accept USDC through the x402 protocol on Base. The terminal handles the menu, touch input, and vend relay; a TypeScript service creates payment sessions and sends signed payments to an external facilitator for settlement. This split keeps blockchain logic and payment credentials off the vending hardware.

> [!IMPORTANT]
> This repository is a prototype. The backend implements the payment flow but keeps sessions in memory, and most firmware peripherals remain scaffolds. Do not use it unattended or with production funds.

## How it works

```text
Customer wallet ── x402/HTTPS ──> Backend ── verify + settle ──> Facilitator
                                    ^
                                    │ menu, session, status
                                    │
                              ESP32-S3 terminal ──> display / touch / relay
```

The facilitator verifies the wallet's signed USDC authorization and submits the transfer on Base.

1. The terminal fetches the menu from `GET /api/menu`.
2. A customer selects an item, and the terminal creates a payment session with `POST /api/session`.
3. The backend returns a payment URL for the terminal to render as a QR code.
4. The wallet requests that URL, receives an HTTP `402` response, signs the USDC authorization, and retries with `PAYMENT-SIGNATURE`.
5. The backend verifies and settles the payment through the facilitator.
6. The terminal polls the session status and fires the vend relay after confirmation.

The backend requests x402 v2 payments with the `exact` scheme, Base mainnet (`eip155:8453`), and the Base USDC contract.

## Project status

| Area | Status |
| --- | --- |
| Menu, session, and status APIs | Implemented |
| x402 payment requirements, verification, and settlement | Implemented |
| Backend integration tests | 12 passing tests |
| Wi-Fi connection helper | Implemented; `main` does not call it |
| Display, touch, API client, relay, and firmware state machine | Scaffolded |
| Persistent sessions, authentication, and production deployment | Not implemented |

## Repository layout

```text
backend/     Express service, in-memory sessions, and Vitest tests
firmware/    Rust ESP-IDF application for the ESP32-S3
docs/        System design, wiring, and x402 payment-flow specification
bom.md       Prototype bill of materials
AGENTS.md    Contributor conventions and validation commands
```

## Run the backend

### Requirements

- Node.js 18 or newer
- npm

Install dependencies and create local configuration:

```bash
git clone https://github.com/modeofO/402-hardware.git
cd 402-hardware/backend
npm ci
cp .env.example .env
```

Set `PAYMENT_RECIPIENT` in `.env` to the wallet that should receive USDC. The code falls back to the zero address when this variable is absent, so configure it before attempting a real payment. `PORT` defaults to `3000`.

Start the development server:

```bash
npm run dev
```

Confirm that it is running:

```bash
curl http://localhost:3000/health
curl http://localhost:3000/api/menu
curl -X POST http://localhost:3000/api/session \
  -H 'Content-Type: application/json' \
  -d '{"item_id":"1"}'
```

The final command returns a `session_id` and `payment_url`. Requesting the payment URL without a signature returns `402 Payment Required` with a base64-encoded `PAYMENT-REQUIRED` header.

### API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Check backend health |
| `GET` | `/api/menu` | List vendable items and USDC prices |
| `POST` | `/api/session` | Create a session from `{ "item_id": "1" }` |
| `GET` | `/api/session/:id/status` | Poll `pending`, `confirmed`, or `failed` |
| `GET` | `/pay/:sessionId` | Negotiate and settle an x402 payment |

## Build and flash the firmware

The firmware targets `xtensa-esp32s3-espidf` and pins ESP-IDF v5.3 in `firmware/.cargo/config.toml`.

Install the Xtensa toolchain and flashing utilities:

```bash
cargo install espup --locked
espup install
cargo install espflash --locked
cargo install ldproxy --locked
```

Follow the shell activation instruction printed by `espup`, then connect the ESP32-S3 and run:

```bash
cd firmware
cargo run --release
```

The Cargo runner builds, flashes, and opens the serial monitor. The firmware logs its scaffold state and waits; it does not render the menu or vend an item yet.

See the [design specification](docs/superpowers/specs/2026-06-23-x402-vending-terminal-design.md) for the state machine, GPIO assignments, wiring, and planned implementation. See the [bill of materials](bom.md) for prototype hardware.

## Test and validate changes

```bash
cd backend
npm test
npm run build

cd ../firmware
cargo fmt --check
cargo check
```

Backend tests use Vitest and Supertest. They mock facilitator traffic and must never settle live funds. Firmware validation requires the `esp` toolchain installed by `espup`; describe on-device display, touch, and relay checks in the pull request.

## Known limitations

- Restarting the backend deletes every session.
- Both the menu and session routes define the menu.
- `FACILITATOR_URL` appears in `.env.example`, but the service uses the constant in `backend/src/x402.ts`.
- The backend has no authentication, terminal identity, rate limiting, or durable audit log.
- The project targets Base mainnet and offers no testnet mode.
- The repository has no deployment manifest or automated CI workflow.

## Contributing

Read [AGENTS.md](AGENTS.md) before making changes. Keep backend tests isolated from live payment infrastructure, run the relevant validation commands, and document hardware verification with logs or photos. Use focused commits with the existing `feat:`, `fix:`, or `scaffold:` prefixes where appropriate.

## License

This repository lacks a license. Until the maintainers add one, no permission is granted to copy, modify, or redistribute the code.

Last reviewed: July 13, 2026.
