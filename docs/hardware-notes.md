# Hardware Bring-up Notes

Findings that cost real time on the bench. Everything here was verified on the
ESP32-S3-DEVKITC-1-N32R16V plus Adafruit 2050 (HX8357D 3.5" TFT) and is
independent of which application the board runs. Read this before touching
`firmware/src/display.rs`, `touch.rs`, or the wiring.

Source of truth for the code: commit `c4c0170` ("bring up HX8357D display and
resistive touch on real hardware") and the header comments in `display.rs`.

## Display: how the encoding was figured out

The panel stayed black through several rounds. What it actually took, in the
order the problems were found:

1. **Board ships in 8-bit parallel mode, not SPI.** Adafruit's page implies SPI
   by default; the 2050 does not. IM2 must be tied to 3.3V: solder-close the
   IM2 jumper on the back, or jumper the IM2 breakout pin to 3.3Vo (never 5V).
   Until this is done the SPI header is inert and no software change helps.
2. **Every control line was verified with a meter** before trusting software.
   `firmware/src/bin/pin_blink.rs` toggles CLK/MOSI/CS/DC/RST together at
   ~0.5Hz; any pin that sits still at the display header solder joint is the
   broken path. Run with `cargo run --bin pin_blink`.
3. **No MISO.** Passing GPIO13 as the SPI bus MISO pin made the panel ignore
   all traffic, probe-verified. Root cause inside the esp-idf full-duplex path
   is unknown. The bus is created with `None` for MISO and the panel is driven
   write-only. Leave the MISO pad unconnected.
4. **Manual chip select.** Hardware CS from the SPI driver releases between the
   command byte and its parameters; the HX8357D needs CS held low across the
   whole command+data window (as Adafruit's C driver does). CS is a plain GPIO
   toggled in `Display::cmd`, and the SPI device is created with no CS pin.
5. **Init sequence is Adafruit's, verbatim.** `mipidsi` has no HX8357D model so
   `display.rs` is a minimal custom driver. The SETC / SETRGB / SETCOM / SETOSC
   / SETPANEL / SETPWR1 / SETSTBA / SETCYC / SETGAMMA parameter bytes are copied
   from `Adafruit_HX8357.cpp`. Do not "tidy" them.
6. **Pixel format: RGB565, big-endian on the wire.** `COLMOD = 0x55` selects
   16-bit colour. Each pixel goes out high byte first (`to_be_bytes()` in
   `DrawTarget::draw_iter` and `clear`). Little-endian gives scrambled colours
   that look almost right, which is the trap.
7. **Orientation: `MADCTL = 0xA0`** (MY | MV) gives landscape 480x320 with the
   USB port on the left. Touch axes are swapped to match (`SWAP_XY = true` in
   `touch.rs`).
8. **Full-frame framebuffer in PSRAM.** 480x320x2 = 307KB, allocated with
   `vec!` and landing in PSRAM via `CONFIG_SPIRAM_USE_MALLOC`. `flush()` sets a
   full-screen window (CASET/PASET) and streams the buffer in one RAMWR burst;
   esp-idf-hal chunks it into max-transfer-sized transactions. Per-pixel SPI
   transactions were orders of magnitude too slow. One flush at 26MHz is about
   100ms.

## Touch (4-wire resistive, same header)

- Read Adafruit-TouchScreen style: drive one plate, ADC-sample the other,
  plus a Z (pressure) phase for touch detection. Pins swap between GPIO output
  and ADC input on every read, so short-lived `PinDriver`s borrow the pins per
  phase.
- Sense pins must be on ADC1 (GPIO 1–10). YP=4, XP=5, YM=6, XM=7.
- `RAW_*_MIN/MAX` and `INVERT_*` in `touch.rs` are the calibration knobs; log
  `Touch: raw=` at debug level and adjust from the corners.

## ESP32-S3 module gotchas

- No GPIO 22–25 on the S3. GPIO 26–32 are flash, GPIO 33–37 are octal PSRAM on
  the N32R16V. Do not assign them.
- Flash size is 32MB (`CONFIG_ESPTOOLPY_FLASHSIZE_32MB`); the scaffold had 16MB
  and the image would not boot.
- probe-rs opens the USB-JTAG probe but its flash algorithm times out on the
  32MB octal flash. Flash with `espflash flash --monitor` (wired as the cargo
  runner); keep probe-rs for halting/debugging only.
- Console is USB-Serial-JTAG (`CONFIG_ESP_CONSOLE_USB_SERIAL_JTAG`), so logs
  arrive on the same USB cable that flashes.
- `sdkconfig.defaults` is applied through `ESP_IDF_SDKCONFIG_DEFAULTS` in
  `.cargo/config.toml`; editing the file alone without that env var did
  nothing.
- Windows builds need `ESP_IDF_TOOLS_INSTALL_DIR=global` and a short
  `CARGO_TARGET_DIR` (e.g. `C:\esp\t`) or CMake fails with "Too long output
  directory".
