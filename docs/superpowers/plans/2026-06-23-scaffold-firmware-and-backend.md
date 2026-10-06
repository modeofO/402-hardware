# x402 Vending Terminal — Scaffold Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up both the ESP32-S3 Rust firmware and TypeScript cloud backend projects so they compile, run, and communicate over a shared API contract.

**Architecture:** Two projects in one repo — `firmware/` (Rust, esp-idf-svc) and `backend/` (TypeScript, Express). The backend serves a menu API and handles x402 payment sessions. The firmware fetches the menu, displays items, renders QR codes, and polls for payment confirmation. Scaffolding gets both projects building with stub implementations for all modules.

**Tech Stack:**
- Firmware: Rust (esp-rs std path), esp-idf-svc, embedded-graphics, qrcode crate
- Backend: TypeScript, Express, @x402/core, @x402/evm, viem
- Toolchain: espup, probe-rs, Node.js 20+

## Global Constraints

- ESP32-S3 target: `xtensa-esp32s3-espidf`
- ESP-IDF version: v5.3
- Rust edition: 2021
- Node.js: 20+
- Chain: Base mainnet (eip155:8453)
- USDC contract: `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`
- x402 version: 2, scheme: exact

---

## File Structure

### Firmware (`firmware/`)

```
firmware/
├── .cargo/
│   └── config.toml              # Target, linker, build-std config
├── Cargo.toml                   # Dependencies
├── sdkconfig.defaults           # ESP-IDF options (WiFi, SPI, partition size)
├── build.rs                     # embuild integration
├── rust-toolchain.toml          # Pin ESP Rust toolchain channel
├── src/
│   ├── main.rs                  # Entry point, state machine, module wiring
│   ├── wifi.rs                  # WiFi connect/reconnect
│   ├── display.rs               # SPI display driver, rendering helpers
│   ├── touch.rs                 # Resistive touchscreen input (stub)
│   ├── api.rs                   # HTTP client — menu fetch, session create, poll status
│   ├── vend.rs                  # Relay GPIO control
│   └── types.rs                 # Shared types (MenuItem, Session, PaymentStatus)
```

### Backend (`backend/`)

```
backend/
├── package.json
├── tsconfig.json
├── .env.example                 # Required env vars documented
├── src/
│   ├── index.ts                 # Express server entry point
│   ├── routes/
│   │   ├── menu.ts              # GET /api/menu
│   │   ├── session.ts           # POST /api/session, GET /api/session/:id/status
│   │   └── pay.ts               # GET /pay/:sessionId (x402 402 flow)
│   ├── session-store.ts         # In-memory session management
│   ├── x402.ts                  # x402 payment middleware setup
│   └── types.ts                 # Shared types (MenuItem, Session, PaymentStatus)
├── test/
│   ├── menu.test.ts             # Menu endpoint tests
│   ├── session.test.ts          # Session lifecycle tests
│   └── pay.test.ts              # x402 payment flow tests
```

---

### Task 1: Scaffold the Rust firmware project

**Files:**
- Create: `firmware/.cargo/config.toml`
- Create: `firmware/Cargo.toml`
- Create: `firmware/sdkconfig.defaults`
- Create: `firmware/build.rs`
- Create: `firmware/rust-toolchain.toml`
- Create: `firmware/src/main.rs`
- Create: `firmware/src/types.rs`

**Interfaces:**
- Produces: Compilable Rust ESP-IDF project with `MenuItem`, `Session`, `PaymentStatus` types used by all firmware modules

- [ ] **Step 1: Create `.cargo/config.toml`**

```toml
[build]
target = "xtensa-esp32s3-espidf"

[target.xtensa-esp32s3-espidf]
linker = "ldproxy"
runner = "espflash flash --monitor"

[unstable]
build-std = ["std", "panic_abort"]

[env]
MCU = "esp32s3"
ESP_IDF_VERSION = "v5.3"
```

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "esp"
```

- [ ] **Step 3: Create `Cargo.toml`**

```toml
[package]
name = "x402-terminal"
version = "0.1.0"
edition = "2021"

[dependencies]
esp-idf-svc = { version = "0.50", features = ["critical-section"] }
embedded-graphics = "0.8"
qrcode = "0.14"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"

[build-dependencies]
embuild = "0.33"
```

- [ ] **Step 4: Create `sdkconfig.defaults`**

```
CONFIG_ESP_WIFI_ENABLED=y
CONFIG_ESP_WIFI_MODE_STA=y
CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y
CONFIG_PARTITION_TABLE_CUSTOM=n
CONFIG_PARTITION_TABLE_SINGLE_APP_LARGE=y
CONFIG_ESP_MAIN_TASK_STACK_SIZE=32768
CONFIG_SPIRAM=y
CONFIG_SPIRAM_MODE_OCT=y
CONFIG_SPIRAM_SPEED_80M=y
```

- [ ] **Step 5: Create `build.rs`**

```rust
fn main() {
    embuild::espidf::sysenv::output();
}
```

- [ ] **Step 6: Create `src/types.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub price_usdc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub payment_url: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStatus {
    pub status: PaymentStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalState {
    Boot,
    FetchMenu,
    Idle,
    ItemSelected(usize),
    AwaitingPayment(String),
    Dispensing,
}
```

- [ ] **Step 7: Create `src/main.rs`**

```rust
mod api;
mod display;
mod touch;
mod types;
mod vend;
mod wifi;

use log::info;
use types::TerminalState;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("x402 vending terminal starting");
    info!("State: {:?}", TerminalState::Boot);

    // TODO: Initialize peripherals and enter state machine loop
    info!("Scaffold complete — modules loaded, waiting for implementation");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

- [ ] **Step 8: Verify firmware compiles**

Run: `cd firmware && cargo check 2>&1 | tail -20`

Expected: Compilation will fail because module files don't exist yet. That's expected — we create them in the next tasks.

- [ ] **Step 9: Commit**

```bash
git add firmware/
git commit -m "scaffold: init Rust ESP-IDF firmware project structure"
```

---

### Task 2: Add firmware module stubs

**Files:**
- Create: `firmware/src/wifi.rs`
- Create: `firmware/src/display.rs`
- Create: `firmware/src/touch.rs`
- Create: `firmware/src/api.rs`
- Create: `firmware/src/vend.rs`

**Interfaces:**
- Consumes: `types::MenuItem`, `types::Session`, `types::SessionStatus`, `types::PaymentStatus` from Task 1
- Produces: Module stubs with public function signatures that `main.rs` and other modules will call

- [ ] **Step 1: Create `src/wifi.rs`**

```rust
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::info;

pub fn connect(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspDefaultNvsPartition>,
    ssid: &str,
    password: &str,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), nvs)?,
        sysloop,
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: password.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    info!("WiFi connected");
    Ok(wifi)
}
```

- [ ] **Step 2: Create `src/display.rs`**

```rust
use log::info;

pub struct Display;

impl Display {
    pub fn init() -> anyhow::Result<Self> {
        info!("Display: stub init");
        Ok(Self)
    }

    pub fn show_message(&mut self, msg: &str) {
        info!("Display: {}", msg);
    }

    pub fn show_menu(&mut self, items: &[crate::types::MenuItem]) {
        info!("Display: showing {} menu items", items.len());
        for item in items {
            info!("  {} — {} USDC", item.name, item.price_usdc);
        }
    }

    pub fn show_qr(&mut self, data: &str) {
        info!("Display: QR code for {}", data);
    }
}
```

- [ ] **Step 3: Create `src/touch.rs`**

```rust
use log::info;

pub struct Touch;

impl Touch {
    pub fn init() -> anyhow::Result<Self> {
        info!("Touch: stub init");
        Ok(Self)
    }

    pub fn poll(&self) -> Option<usize> {
        None
    }
}
```

- [ ] **Step 4: Create `src/api.rs`**

```rust
use crate::types::{MenuItem, Session, SessionStatus};
use log::info;

pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn fetch_menu(&self) -> anyhow::Result<Vec<MenuItem>> {
        info!("API: fetching menu from {}/api/menu", self.base_url);
        Ok(vec![])
    }

    pub fn create_session(&self, item_id: &str) -> anyhow::Result<Session> {
        info!("API: creating session for item {}", item_id);
        anyhow::bail!("not implemented")
    }

    pub fn poll_status(&self, session_id: &str) -> anyhow::Result<SessionStatus> {
        info!("API: polling status for session {}", session_id);
        anyhow::bail!("not implemented")
    }
}
```

- [ ] **Step 5: Create `src/vend.rs`**

```rust
use log::info;

pub struct Vend;

impl Vend {
    pub fn init() -> anyhow::Result<Self> {
        info!("Vend: stub init (relay on GPIO 26)");
        Ok(Self)
    }

    pub fn dispense(&mut self, pulse_ms: u64) {
        info!("Vend: firing relay for {}ms", pulse_ms);
        std::thread::sleep(std::time::Duration::from_millis(pulse_ms));
        info!("Vend: relay off");
    }
}
```

- [ ] **Step 6: Add `anyhow` dependency to `Cargo.toml`**

Add to `[dependencies]` in `firmware/Cargo.toml`:

```toml
anyhow = "1"
```

- [ ] **Step 7: Verify firmware compiles**

Run: `cd firmware && cargo check 2>&1 | tail -20`

Expected: `Finished` with no errors. Warnings about unused code are fine at this stage.

- [ ] **Step 8: Commit**

```bash
git add firmware/
git commit -m "scaffold: add firmware module stubs (wifi, display, touch, api, vend)"
```

---

### Task 3: Scaffold the TypeScript backend project

**Files:**
- Create: `backend/package.json`
- Create: `backend/tsconfig.json`
- Create: `backend/.env.example`
- Create: `backend/src/types.ts`
- Create: `backend/src/session-store.ts`
- Create: `backend/src/index.ts`

**Interfaces:**
- Produces: Running Express server with health endpoint, `SessionStore` class, shared types matching firmware types

- [ ] **Step 1: Create `backend/package.json`**

```json
{
  "name": "x402-terminal-backend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc",
    "start": "node dist/index.js",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "express": "^5.1.0",
    "@x402/core": "latest",
    "@x402/evm": "latest",
    "viem": "^2.0.0",
    "dotenv": "^16.4.0"
  },
  "devDependencies": {
    "@types/express": "^5.0.0",
    "@types/node": "^22.0.0",
    "tsx": "^4.0.0",
    "typescript": "^5.7.0",
    "vitest": "^3.0.0",
    "supertest": "^7.0.0"
  }
}
```

- [ ] **Step 2: Create `backend/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "moduleResolution": "Node16",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true
  },
  "include": ["src"],
  "exclude": ["node_modules", "dist", "test"]
}
```

- [ ] **Step 3: Create `backend/.env.example`**

```
PORT=3000
PAYMENT_RECIPIENT=0xYOUR_WALLET_ADDRESS
FACILITATOR_URL=https://x402.org/facilitator
```

- [ ] **Step 4: Create `backend/src/types.ts`**

```typescript
export interface MenuItem {
  id: string;
  name: string;
  price_usdc: string;
}

export interface Session {
  session_id: string;
  item: MenuItem;
  payment_url: string;
  status: PaymentStatus;
  created_at: number;
}

export type PaymentStatus = "pending" | "confirmed" | "failed";

export interface SessionStatus {
  status: PaymentStatus;
}
```

- [ ] **Step 5: Create `backend/src/session-store.ts`**

```typescript
import { randomUUID } from "crypto";
import type { MenuItem, Session, PaymentStatus } from "./types.js";

export class SessionStore {
  private sessions = new Map<string, Session>();

  create(item: MenuItem, baseUrl: string): Session {
    const session_id = randomUUID();
    const session: Session = {
      session_id,
      item,
      payment_url: `${baseUrl}/pay/${session_id}`,
      status: "pending",
      created_at: Date.now(),
    };
    this.sessions.set(session_id, session);
    return session;
  }

  get(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  updateStatus(id: string, status: PaymentStatus): void {
    const session = this.sessions.get(id);
    if (session) {
      session.status = status;
    }
  }
}
```

- [ ] **Step 6: Create `backend/src/index.ts`**

```typescript
import express from "express";
import dotenv from "dotenv";
import { menuRouter } from "./routes/menu.js";
import { sessionRouter } from "./routes/session.js";
import { payRouter } from "./routes/pay.js";
import { SessionStore } from "./session-store.js";

dotenv.config();

const app = express();
const port = parseInt(process.env.PORT || "3000");

app.use(express.json());

const store = new SessionStore();
app.locals.store = store;

app.get("/health", (_req, res) => {
  res.json({ status: "ok" });
});

app.use("/api", menuRouter);
app.use("/api", sessionRouter);
app.use("/", payRouter);

app.listen(port, () => {
  console.log(`x402 terminal backend listening on port ${port}`);
});

export { app };
```

- [ ] **Step 7: Install dependencies**

Run: `cd backend && npm install`

Expected: `added N packages` with no errors

- [ ] **Step 8: Verify TypeScript compiles**

Run: `cd backend && npx tsc --noEmit 2>&1 | tail -20`

Expected: Will fail because route files don't exist yet. That's expected.

- [ ] **Step 9: Commit**

```bash
git add backend/
git commit -m "scaffold: init TypeScript backend with Express, session store, and types"
```

---

### Task 4: Add backend route stubs and tests

**Files:**
- Create: `backend/src/routes/menu.ts`
- Create: `backend/src/routes/session.ts`
- Create: `backend/src/routes/pay.ts`
- Create: `backend/src/x402.ts`
- Create: `backend/test/menu.test.ts`
- Create: `backend/test/session.test.ts`
- Create: `backend/test/pay.test.ts`

**Interfaces:**
- Consumes: `SessionStore` from Task 3, types from `types.ts`
- Produces: Working API endpoints matching the firmware's `api.rs` expectations:
  - `GET /api/menu` → `MenuItem[]`
  - `POST /api/session` `{ item_id }` → `{ session_id, payment_url }`
  - `GET /api/session/:id/status` → `{ status }`
  - `GET /pay/:sessionId` → 402 response with x402 payment requirements

- [ ] **Step 1: Write failing test for menu endpoint**

Create `backend/test/menu.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("GET /api/menu", () => {
  it("returns a list of menu items", async () => {
    const res = await request(app).get("/api/menu");
    expect(res.status).toBe(200);
    expect(Array.isArray(res.body)).toBe(true);
    expect(res.body.length).toBeGreaterThan(0);
    expect(res.body[0]).toHaveProperty("id");
    expect(res.body[0]).toHaveProperty("name");
    expect(res.body[0]).toHaveProperty("price_usdc");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd backend && npx vitest run test/menu.test.ts 2>&1 | tail -10`

Expected: FAIL — route doesn't exist yet

- [ ] **Step 3: Create `backend/src/routes/menu.ts`**

```typescript
import { Router } from "express";
import type { MenuItem } from "../types.js";

export const menuRouter = Router();

const MENU: MenuItem[] = [
  { id: "1", name: "Soda", price_usdc: "1.50" },
  { id: "2", name: "Water", price_usdc: "1.00" },
  { id: "3", name: "Snack", price_usdc: "2.00" },
];

menuRouter.get("/menu", (_req, res) => {
  res.json(MENU);
});
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd backend && npx vitest run test/menu.test.ts 2>&1 | tail -10`

Expected: PASS

- [ ] **Step 5: Write failing test for session endpoints**

Create `backend/test/session.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("POST /api/session", () => {
  it("creates a session for a valid item", async () => {
    const res = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    expect(res.status).toBe(201);
    expect(res.body).toHaveProperty("session_id");
    expect(res.body).toHaveProperty("payment_url");
  });

  it("returns 404 for invalid item", async () => {
    const res = await request(app)
      .post("/api/session")
      .send({ item_id: "999" });
    expect(res.status).toBe(404);
  });
});

describe("GET /api/session/:id/status", () => {
  it("returns pending status for new session", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const res = await request(app).get(`/api/session/${sessionId}/status`);
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("pending");
  });

  it("returns 404 for unknown session", async () => {
    const res = await request(app).get("/api/session/nonexistent/status");
    expect(res.status).toBe(404);
  });
});
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cd backend && npx vitest run test/session.test.ts 2>&1 | tail -10`

Expected: FAIL

- [ ] **Step 7: Create `backend/src/routes/session.ts`**

```typescript
import { Router } from "express";
import type { SessionStore } from "../session-store.js";

const MENU_IDS = new Set(["1", "2", "3"]);

const MENU = [
  { id: "1", name: "Soda", price_usdc: "1.50" },
  { id: "2", name: "Water", price_usdc: "1.00" },
  { id: "3", name: "Snack", price_usdc: "2.00" },
];

export const sessionRouter = Router();

sessionRouter.post("/session", (req, res) => {
  const { item_id } = req.body;
  if (!item_id || !MENU_IDS.has(item_id)) {
    res.status(404).json({ error: "item not found" });
    return;
  }

  const item = MENU.find((m) => m.id === item_id)!;
  const store = req.app.locals.store as SessionStore;
  const baseUrl = `${req.protocol}://${req.get("host")}`;
  const session = store.create(item, baseUrl);

  res.status(201).json({
    session_id: session.session_id,
    payment_url: session.payment_url,
  });
});

sessionRouter.get("/session/:id/status", (req, res) => {
  const store = req.app.locals.store as SessionStore;
  const session = store.get(req.params.id);
  if (!session) {
    res.status(404).json({ error: "session not found" });
    return;
  }
  res.json({ status: session.status });
});
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cd backend && npx vitest run test/session.test.ts 2>&1 | tail -10`

Expected: PASS

- [ ] **Step 9: Create `backend/src/x402.ts` (stub)**

```typescript
export const USDC_BASE = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
export const BASE_NETWORK = "eip155:8453";
export const X402_VERSION = 2;
```

- [ ] **Step 10: Write failing test for pay endpoint**

Create `backend/test/pay.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import request from "supertest";
import { app } from "../src/index.js";

describe("GET /pay/:sessionId", () => {
  it("returns 402 with payment requirements for valid session", async () => {
    const create = await request(app)
      .post("/api/session")
      .send({ item_id: "1" });
    const sessionId = create.body.session_id;

    const res = await request(app).get(`/pay/${sessionId}`);
    expect(res.status).toBe(402);

    const paymentRequired = JSON.parse(
      Buffer.from(res.headers["payment-required"], "base64").toString()
    );
    expect(paymentRequired.x402Version).toBe(2);
    expect(paymentRequired.accepts).toHaveLength(1);
    expect(paymentRequired.accepts[0].scheme).toBe("exact");
    expect(paymentRequired.accepts[0].network).toBe("eip155:8453");
  });

  it("returns 404 for unknown session", async () => {
    const res = await request(app).get("/pay/nonexistent");
    expect(res.status).toBe(404);
  });
});
```

- [ ] **Step 11: Run test to verify it fails**

Run: `cd backend && npx vitest run test/pay.test.ts 2>&1 | tail -10`

Expected: FAIL

- [ ] **Step 12: Create `backend/src/routes/pay.ts`**

```typescript
import { Router } from "express";
import type { SessionStore } from "../session-store.js";
import { USDC_BASE, BASE_NETWORK, X402_VERSION } from "../x402.js";

export const payRouter = Router();

payRouter.get("/pay/:sessionId", (req, res) => {
  const store = req.app.locals.store as SessionStore;
  const session = store.get(req.params.sessionId);
  if (!session) {
    res.status(404).json({ error: "session not found" });
    return;
  }

  const payTo = process.env.PAYMENT_RECIPIENT || "0x0000000000000000000000000000000000000000";

  const amountAtomic = Math.round(
    parseFloat(session.item.price_usdc) * 1_000_000
  ).toString();

  const paymentRequired = {
    x402Version: X402_VERSION,
    error: "Payment required",
    resource: {
      url: session.payment_url,
      description: session.item.name,
      mimeType: "application/json",
    },
    accepts: [
      {
        scheme: "exact",
        network: BASE_NETWORK,
        amount: amountAtomic,
        asset: USDC_BASE,
        payTo,
        maxTimeoutSeconds: 120,
        extra: {
          name: "USDC",
          version: "2",
        },
      },
    ],
    extensions: {},
  };

  const encoded = Buffer.from(JSON.stringify(paymentRequired)).toString(
    "base64"
  );

  res.status(402).set("PAYMENT-REQUIRED", encoded).json(paymentRequired);
});
```

- [ ] **Step 13: Run all tests**

Run: `cd backend && npx vitest run 2>&1 | tail -20`

Expected: All tests PASS

- [ ] **Step 14: Verify dev server starts**

Run: `cd backend && timeout 5 npx tsx src/index.ts 2>&1 || true`

Expected: `x402 terminal backend listening on port 3000`

- [ ] **Step 15: Commit**

```bash
git add backend/
git commit -m "scaffold: add backend route stubs with menu, session, and x402 pay endpoints"
```

---

### Task 5: Update .gitignore and push

**Files:**
- Modify: `.gitignore`

**Interfaces:**
- Consumes: All files from Tasks 1-4
- Produces: Clean repo pushed to remote

- [ ] **Step 1: Update `.gitignore`**

```
.env
node_modules/
dist/
target/
```

- [ ] **Step 2: Verify both projects**

Run firmware check: `cd firmware && cargo check 2>&1 | tail -5`
Run backend tests: `cd backend && npx vitest run 2>&1 | tail -5`

Expected: Both pass

- [ ] **Step 3: Commit and push**

```bash
git add .gitignore
git commit -m "update gitignore for firmware and backend build artifacts"
git push
```
