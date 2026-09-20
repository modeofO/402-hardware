# Lamp Timer — Design

Repurposes the x402 vending terminal hardware as a countdown timer for a table
lamp. The ESP32-S3, 3.5" touch display, and firmware display/touch drivers are
reused unchanged; the payment flow, Wi-Fi, and backend are dropped on this
branch. The lamp's brightness stays on its own 3-way knob; the timer only
controls on/off.

## Hardware

| Component | Role |
|-----------|------|
| ESP32-S3-DEVKITC-1-N32R16V | Timer logic, display, touch |
| Adafruit 2050 HX8357D 480x320 touch TFT | Countdown display and buttons |
| Digital Loggers IoT Power Relay | Switches the lamp's mains |
| GE 3-way bulb (30/70/100W or LED equivalent) | Load; brightness via lamp knob |

The Adafruit 2895 relay FeatherWing from the vending build is **not used**:
it is rated 0.5A @ 120VAC and has no mains creepage/clearance. The IoT Power
Relay is a sealed, UL-listed box (12A, opto-isolated trigger, MOV surge
suppression, input debounce) so no mains wiring is exposed anywhere.

## Wiring

Display and touch: unchanged from the vending terminal, see
`docs/hardware-notes.md` and `firmware/src/pins.rs`.

### ESP32-S3 → IoT Power Relay

| ESP32-S3 Pin | IoT Relay terminal | Note |
|--------------|--------------------|------|
| GPIO 21 | trigger `+` | 3.3V logic is inside the 3–60V DC input range |
| GND | trigger `-` | |

Lamp plugs into one of the **"normally OFF"** outlets. GPIO high = lamp on.
The ESP32 runs from USB-C; the IoT Relay's "always on" outlet can feed the
USB supply so the whole thing is one wall plug.

## UI

Landscape 480x320, layout in `firmware/src/ui.rs`:

- Seven-segment countdown across the top, `MM:SS` (or `H:MM:SS` past an hour).
  Green while the lamp is on, white when set and idle, orange at "DONE".
- Status line: `LAMP ON` / `LAMP OFF` / `SET A TIME` / `DONE - LAMP OFF`.
- Row of four: `+1 MIN`, `+5 MIN`, `+15 MIN`, `CLEAR`.
- Full-width `START` / `STOP`.

Adding time while running extends the deadline in place. STOP pauses (keeps
the remaining time); START resumes. CLEAR zeroes and switches the lamp off.
Boot preset is 15 minutes; maximum is 9:59:59.

## State machine (`firmware/src/timer.rs`)

```
Idle(secs) --START, secs>0--> Running(ends_at) --deadline--> Done
Running --STOP--> Idle(remaining)
Running --+N--> Running(ends_at + N)
Idle/Done --+N--> Idle(secs + N)
any --CLEAR--> Idle(0)
Done --START--> Running(last_started)
```

The main loop ticks the timer, maps touches to buttons, then forces the relay
to match `is_running()` every iteration. The screen is redrawn only when
`(remaining, phase)` changes, i.e. once a second while running.

## Not in scope (yet)

- Wall-clock schedules (would need Wi-Fi + NTP; `wifi.rs` is in git history).
- Brightness control. A 3-way socket needs two switched hots; the knob does
  it by hand for now.
- Backlight dimming when idle (Lite pin is unwired).
