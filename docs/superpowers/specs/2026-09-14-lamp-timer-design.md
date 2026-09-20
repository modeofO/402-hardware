# Lamp Timer — Design

Repurposes the x402 vending terminal hardware as a timer switch for a table
lamp: the lamp follows a daily on/off schedule against a clock set on the
touchscreen. (The first version was a tap-to-set countdown; see git history.)
The ESP32-S3, 3.5" touch display, and firmware display/touch drivers are
reused unchanged; the payment flow, Wi-Fi, and backend are dropped on this
branch. The lamp's brightness stays on its own 3-way knob; the timer only
controls on/off.

## Hardware

| Component | Role |
|-----------|------|
| ESP32-S3-DEVKITC-1-N32R16V | Timer logic, display, touch |
| Adafruit 2050 HX8357D 480x320 touch TFT | Clock display and buttons |
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

Landscape 480x320, layout in `firmware/src/ui.rs`. Two screens share one grid:
seven-segment `HH:MM` across the top, a status line, a detail line, and two
button rows.

**Main screen**

- Clock digits: green while the lamp is on, white when off, orange when the
  time was restored after a power cut, grey dashes while the clock is unset.
- Status line: `LAMP ON` / `LAMP OFF`, with ` - MANUAL` during an override.
- Detail line: `ON 17:00 - OFF 00:00 DAILY`, or a clock warning
  (`CLOCK NOT SET - TAP SET CLOCK` / `POWER WAS LOST - CHECK THE CLOCK`).
- Row of two: `SET CLOCK`, `SCHEDULE`. Full-width `TURN LAMP ON` / `OFF`.

**Editor screen** (SET CLOCK, and SCHEDULE as two steps: on time, then off time)

- Row of four: `HOUR -`, `HOUR +`, `MIN -`, `MIN +`. Holding one repeats it.
  The clock moves in 1-minute steps, schedule times in 5-minute steps; both
  wrap around midnight.
- `CANCEL` and `SAVE` (`NEXT` on the first schedule step).

## Clock (`firmware/src/clock.rs`, `store.rs`)

No Wi-Fi and no RTC chip: the user sets the time on the touchscreen and the
ESP32's 40MHz crystal keeps it, drifting on the order of a second a day. Only
the time of day is tracked — no date, weekday, time zone, or DST; adjust the
clock by hand when DST changes.

The time lives in the ESP-IDF system clock, which survives crashes and
reflashes but not loss of power. The firmware checkpoints the time of day to
NVS once a minute; after a power-on reset it restores the checkpoint, which is
late by the length of the outage, and shows `POWER WAS LOST - CHECK THE CLOCK`
until the user saves the clock again. The lamp follows the restored time in
the meantime — a schedule that runs late beats one that does not run. With no
checkpoint (first boot) the lamp stays off until the clock is set, except by
manual override.

A battery-backed DS3231 on I2C would remove both the drift and the outage
error; see "Not in scope".

## Lamp logic (`firmware/src/schedule.rs`)

One daily window, saved to NVS. `on > off` spans midnight (`17:00-00:00`,
`22:00-06:00`); `00:00` as the off time means midnight; `on == off` is an empty
window (lamp never on by schedule). Default until one is saved: 17:00–00:00.

```
scheduled = clock set && window covers now
lamp      = manual override, else scheduled

TURN LAMP ON/OFF --> override = !lamp   (or cleared, if that matches schedule)
schedule catches up with override --> override cleared
```

So an override lasts until the next scheduled switch, like a smart plug: turn
the lamp off at 20:00 and it stays off past midnight, then comes on at 17:00
as usual. Saving a new schedule also clears the override.

The main loop polls touch, evaluates the lamp logic, then forces the relay to
match every iteration. The screen is redrawn only when what it shows changes:
once a minute on the main screen, per button press in the editor.

## Not in scope (yet)

- A battery-backed RTC (DS3231, I2C on two free GPIOs) so the clock survives
  power cuts; NTP over Wi-Fi is the other option (`wifi.rs` is in git history).
- Per-weekday schedules or more than one window a day.
- The original countdown timer (`timer.rs` is in git history).
- Brightness control. A 3-way socket needs two switched hots; the knob does
  it by hand for now.
- Backlight dimming when idle (Lite pin is unwired).
