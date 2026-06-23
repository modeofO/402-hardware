# x402 Vending Terminal — Hardware Design Spec

## Overview

A countertop payment terminal that attaches to a vending machine, enabling customers to pay with USDC via the x402 protocol. The terminal displays a QR code, the customer scans it with their phone wallet, and on payment confirmation the terminal triggers the vending machine to dispense.

## Hardware Platform

**MCU:** ESP32-S3 (dev board for breadboard prototype)

Chosen for built-in WiFi, sufficient RAM for QR rendering on a 480x320 display, and strong Rust support via the esp-rs ecosystem.

## Components (Breadboard Prototype)

| Component | Specification | Purpose |
|-----------|--------------|---------|
| ESP32-S3 dev board | ESP32-S3-DevKitC or equivalent | Main MCU — WiFi, processing, GPIO |
| 3.5" TFT display | ILI9488, 480x320, SPI interface | QR code display for customer |
| 5V relay module | Single-channel, optoisolated | Generic vend trigger output |
| USB-C cable | Data + power | Power and flashing via dev board |
| Breadboard | Full-size | Prototyping connections |
| Jumper wires | Male-to-male, male-to-female | Wiring |

## Wiring

### ESP32-S3 → 3.5" TFT Display (SPI)

| ESP32-S3 Pin | Display Pin | Function |
|-------------|-------------|----------|
| GPIO 18 | SCK | SPI Clock |
| GPIO 23 | MOSI | SPI Data |
| GPIO 5 | CS | Chip Select |
| GPIO 4 | DC | Data/Command |
| GPIO 2 | RST | Reset |
| 3.3V | VCC | Power |
| GND | GND | Ground |
| 3.3V | LED | Backlight |

### ESP32-S3 → Relay Module

| ESP32-S3 Pin | Relay Pin | Function |
|-------------|-----------|----------|
| GPIO 26 | IN | Trigger signal |
| 5V (VBUS) | VCC | Relay power |
| GND | GND | Ground |

### Power

Everything runs off the ESP32-S3 dev board's USB-C. The dev board regulates to 3.3V for the MCU and display, and passes through 5V on the VBUS pin for the relay module.

## Software Architecture

### Language & Ecosystem

Rust, using the esp-rs `std` path for full TCP/IP, TLS, and WiFi support.

### Crates

| Crate | Purpose |
|-------|---------|
| `esp-idf-svc` | WiFi, HTTP client, TLS, event loop, SPI, GPIO |
| `embedded-graphics` | 2D rendering for the display |
| `mipidsi` | ILI9488 display driver over SPI |
| `qrcode` | QR code generation |
| `defmt` + `defmt-rtt` | Structured logging via RTT |

### Modules

- **`wifi.rs`** — WiFi connection and reconnection via `esp_idf_svc::wifi`
- **`payment.rs`** — x402 payment request generation, QR encoding, payment verification via `esp_idf_svc::http::client`
- **`display.rs`** — SPI display driver via `esp_idf_svc::hal::spi` + `mipidsi` + `embedded-graphics`, QR rendering, status messages

### State Machine

```
IDLE → AWAITING_PAYMENT → CONFIRMING → DISPENSING → IDLE
```

| State | Behavior |
|-------|----------|
| IDLE | Display "Ready" / idle screen |
| AWAITING_PAYMENT | Generate x402 payment URI, render QR code on display |
| CONFIRMING | Poll/listen for on-chain USDC payment confirmation |
| DISPENSING | Fire relay GPIO high for configurable duration, display "Dispensing..." |

### Toolchain

- `espup` — install the Xtensa Rust compiler and LLVM fork
- `probe-rs` / `cargo-embed` — flash, debug, and RTT logging over the ESP32-S3's built-in USB-JTAG
- `espflash` — fallback serial flashing

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
- Multi-item selection UI
- OTA firmware updates
