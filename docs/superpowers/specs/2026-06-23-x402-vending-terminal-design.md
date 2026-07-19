# x402 Vending Terminal — Design Spec

## Overview

A countertop payment terminal that attaches to a vending machine, enabling customers to pay with USDC via the x402 v2 protocol on Base. The terminal displays a menu on a touchscreen, the customer taps an item, scans the resulting QR code with their wallet, and the x402 payment flow is handled by a cloud backend. On payment confirmation, the terminal triggers the vending machine to dispense.

## System Architecture

```
Customer Phone ──HTTPS──> Cloud Backend <──HTTPS── ESP32 Terminal
                               │
                               v
                         Facilitator
                    (settles USDC on Base)
```

### Components

1. **ESP32 Terminal (firmware)** — displays menu, renders QR codes, listens for payment confirmation, fires relay
2. **Cloud Backend (Railway)** — x402 resource server, facilitator integration, menu API, ESP32 notification
3. **Facilitator** — verifies EIP-3009 signatures and calls `transferWithAuthorization()` on the USDC contract (Coinbase production facilitator or self-hosted)

## x402 v2 Payment Flow

```
Customer Phone                   Cloud Backend                    ESP32 Terminal
      │                               │                               │
      │                               │    Fetch menu on boot         │
      │                               │<──── GET /api/menu ───────────│
      │                               │──── menu items + prices ─────>│
      │                               │                               │
      │                               │              Customer taps item on touchscreen
      │                               │                               │
      │                               │    Create payment session     │
      │                               │<──── POST /api/session ───────│
      │                               │──── { sessionId, url } ──────>│
      │                               │                               │
      │                               │              Terminal displays QR code
      │                               │              (URL: backend/pay/{sessionId})
      │                               │                               │
      │  1. Scan QR code              │                               │
      │──── GET /pay/{sessionId} ────>│                               │
      │<─── 402 + PAYMENT-REQUIRED ───│                               │
      │     (scheme: exact,           │                               │
      │      network: eip155:8453,    │                               │
      │      asset: USDC on Base,     │                               │
      │      amount, payTo)           │                               │
      │                               │                               │
      │  2. Wallet signs EIP-3009     │                               │
      │     transferWithAuthorization │                               │
      │──── retry + PAYMENT-SIGNATURE>│                               │
      │                               │  3. Verify signature          │
      │                               │  4. Settle via facilitator    │
      │                               │     (transferWithAuthorization│
      │                               │      on USDC contract)        │
      │                               │                               │
      │                               │  5. Notify ESP32              │
      │                               │──── payment confirmed ───────>│
      │<─── 200 OK ──────────────────│                               │
      │                               │                         6. Fire relay
      │                               │                            Display "Dispensing..."
```

### x402 Protocol Details

- **x402 Version:** 2
- **Scheme:** exact (fixed-price payment)
- **Network:** eip155:8453 (Base mainnet)
- **Asset:** USDC on Base (`0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`)
- **Signing:** EIP-712 typed data for EIP-3009 `transferWithAuthorization`
- **Headers:** `PAYMENT-REQUIRED` (402 response), `PAYMENT-SIGNATURE` (client retry), `PAYMENT-RESPONSE` (settlement result)

## Hardware Platform

**MCU:** ESP32-S3 (dev board for breadboard prototype)

Chosen for built-in WiFi, sufficient RAM for QR rendering on a 480x320 display, and strong Rust support via the esp-rs ecosystem.

## Components (Breadboard Prototype)

| Component | Specification | Purpose |
|-----------|--------------|---------|
| ESP32-S3 dev board | ESP32-S3-DEVKITC-1-N32R16V (32MB flash, 16MB PSRAM) | Main MCU — WiFi, processing, GPIO |
| 3.5" TFT display | Adafruit 2050, HXD8357D, 480x320, SPI, resistive touch | Menu display, QR codes, touchscreen input |
| Relay module | Adafruit 2895, non-latching mini relay | Vend trigger output |

## Wiring

The display breaks out two interfaces: an SPI header and an 8080-type parallel header. The board ships in SPI mode (solder jumpers on the back select the interface) — wire the SPI side only and leave the parallel side unconnected. Power pins are duplicated on both sides and internally connected; use the SPI side's.

Note on pin choices: the ESP32-S3 has no GPIO 22–25, and on the DEVKITC-1-N32R16V module GPIO 26–32 are used by flash and GPIO 33–37 by the octal PSRAM. SPI pins below are the S3's hardware FSPI defaults; touch pins are on ADC1 (GPIO 1–10).

### ESP32-S3 → 3.5" TFT Display (SPI side)

| ESP32-S3 Pin | Display Pin | Function |
|-------------|-------------|----------|
| GPIO 12 | CLK | SPI Clock |
| GPIO 11 | MOSI | SPI Data (out) |
| GPIO 13 | MISO | SPI Data (in, SD/debug — optional) |
| GPIO 10 | CS | TFT Chip Select |
| GPIO 9 | D/C | Data/Command |
| GPIO 14 | RST | Reset |
| 3.3V | 3-5V (Vin) | Power |
| GND | GND | Ground |

Leave unconnected: **3.3Vo** (regulator output, not an input), **Lite** (backlight, pulled high internally — backlight on by default; wire to a GPIO only if PWM dimming is wanted), **Card CS** (microSD select, unused).

### ESP32-S3 → Resistive Touch (same SPI-side header)

| ESP32-S3 Pin | Display Pin | Function |
|-------------|-------------|----------|
| GPIO 4 | Y+ | Touch sense (ADC1) |
| GPIO 5 | X+ | Touch sense (ADC1) |
| GPIO 6 | Y- | Touch drive |
| GPIO 7 | X- | Touch drive |

### ESP32-S3 → Relay Module

| ESP32-S3 Pin | Relay Pin | Function |
|-------------|-----------|----------|
| GPIO 21 | IN | Trigger signal |
| 5V (VBUS) | VCC | Relay power |
| GND | GND | Ground |

### Power

Everything runs off the ESP32-S3 dev board's USB-C. The dev board regulates to 3.3V for the MCU and display, and passes through 5V on the VBUS pin for the relay module.

## Firmware (ESP32 — Rust)

### Language & Ecosystem

Rust, using the esp-rs `std` path for full TCP/IP, TLS, and WiFi support.

### Crates

| Crate | Purpose |
|-------|---------|
| `esp-idf-svc` | WiFi, HTTP client, TLS, event loop, SPI, GPIO |
| `embedded-graphics` | 2D rendering for the display |
| `mipidsi` | HXD8357D display driver over SPI |
| `qrcode` | QR code generation |
| `defmt` + `defmt-rtt` | Structured logging via RTT |

### Modules

- **`wifi.rs`** — WiFi connection and reconnection via `esp_idf_svc::wifi`
- **`display.rs`** — SPI display driver, QR code rendering, menu UI, status messages
- **`touch.rs`** — Resistive touchscreen input for item selection
- **`api.rs`** — HTTP client for backend communication (fetch menu, create session, poll for payment confirmation)
- **`vend.rs`** — Relay GPIO control, configurable pulse duration

### Firmware State Machine

```
BOOT → FETCH_MENU → IDLE → ITEM_SELECTED → AWAITING_PAYMENT → DISPENSING → IDLE
```

| State | Behavior |
|-------|----------|
| BOOT | Connect to WiFi, initialize display and peripherals |
| FETCH_MENU | GET /api/menu from backend, parse items and prices |
| IDLE | Display menu items on touchscreen, wait for tap |
| ITEM_SELECTED | POST /api/session to backend, render QR code with payment URL |
| AWAITING_PAYMENT | Poll backend for payment confirmation on this session |
| DISPENSING | Fire relay GPIO, display "Dispensing...", return to IDLE |

### Toolchain

- `espup` — install the Xtensa Rust compiler and LLVM fork
- `probe-rs` / `cargo-embed` — flash, debug, and RTT logging over the ESP32-S3's built-in USB-JTAG
- `espflash` — fallback serial flashing

## Cloud Backend

### Responsibilities

1. **Menu API** — serve item list with names and USDC prices to the ESP32
2. **Session management** — create payment sessions tied to a specific item/price/terminal
3. **x402 resource server** — respond with HTTP 402 + `PAYMENT-REQUIRED` header when a customer hits a payment URL
4. **Payment verification** — validate the `PAYMENT-SIGNATURE` header (EIP-712 / EIP-3009)
5. **Settlement** — forward to facilitator to call `transferWithAuthorization()` on the USDC contract
6. **ESP32 notification** — inform the terminal when payment is confirmed (polling endpoint or websocket)

### API Endpoints

| Endpoint | Method | Consumer | Purpose |
|----------|--------|----------|---------|
| `/api/menu` | GET | ESP32 | Fetch menu items and prices |
| `/api/session` | POST | ESP32 | Create a payment session for selected item |
| `/api/session/{id}/status` | GET | ESP32 | Poll for payment confirmation |
| `/pay/{sessionId}` | GET | Customer wallet | x402 payment endpoint (returns 402) |
| `/pay/{sessionId}` | GET | Customer wallet | Retry with PAYMENT-SIGNATURE (settles payment) |

### Deployment

Deployed on Railway (or similar) with a public HTTPS URL. The ESP32 and customer phones both reach it over the internet.

### Facilitator

Uses the Coinbase production facilitator for payment verification and on-chain settlement. The facilitator verifies EIP-712 signatures and executes `transferWithAuthorization()` on the USDC contract on Base.

## Vending Machine Interface

TBD — the relay module provides a generic output for prototyping. The vend signal is a configurable GPIO pulse (duration adjustable). Once the target vending machine is identified, the interface can be adapted to:

- **Pulse/coin simulation** — relay closes for N ms to mimic a coin acceptor signal
- **MDB protocol** — serial communication with the vending machine controller
- **Direct GPIO** — for custom dispense mechanisms

## Connectivity

WiFi only. The ESP32-S3 connects to a configured network on boot. WiFi credentials stored in NVS (non-volatile storage) or hardcoded for the prototype.

## Future (Out of Scope for Prototype)

- Custom PCB design (KiCad)
- Enclosure / mounting
- Cellular fallback
- NFC tap-to-pay
- Multiple payment schemes (upto, batch settlement)
- OTA firmware updates
- Multiple terminal support
- Receipt generation (x402 offer-receipt extension)
