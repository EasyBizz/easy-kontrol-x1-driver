# AI Session Handoff

This file is for future AI sessions to continue development of **Easy KONTROL X1 Driver**.

## Project Summary
- **Goal**: macOS menu bar driver that converts **Native Instruments KONTROL X1** USB events to MIDI.
- **App name**: Easy KONTROL X1 Driver
- **MIDI device name**: EASY KONTROL X1
- **Repo**: https://github.com/EasyBizz/easy-kontrol-x1-driver
- **Latest release**: v1.1.2 (zip contains only the .app at root)

## Current Features
- HID + libusb fallback input handling.
- Full button/knob mapping (including SHIFT latch and FX hold behavior).
- Menu bar app with Start/Stop/ Quit and device list.
- Custom menu bar icon (18x18/36x36 PNG) + app icon (.icns from 1024x1024).
- LED output mapping with working device LEDs.

## Key Behavior Notes
- **SHIFT** is **latching** (press/release toggles). SHIFT LED is internal, independent of DJ software.
- **PLAY/CUE** never get long-press; must behave normally even under SHIFT.
- **Hotcues**: use same channel scheme as other buttons (currently MIDI_CHANNEL_HOTCUE = 0xB0).
- **FX buttons**: hold modifies corresponding knob output on **Channel 3**; short taps send normal CC.
- **Long-press** logic removed for normal buttons (only under SHIFT if needed).

## Mapping Conventions (from current code)
- Primary CCs are on Channel 1 (0xB0), SHIFT-alt on another channel.
- FX-hold modifies knob CCs to same CC on **Channel 3**.
- CUE under SHIFT uses same CC, **Channel 2**.

## LED Mapping Notes
LED indices were enumerated and corrected in `board.yml` (FX LEDs, hotcues, transport LEDs, etc). Mapping is stable.

## Menu Bar / App Build
- Build + bundle: `cargo bundle --release`
- Zip for release: `dist/Easy KONTROL X1 Driver.zip`
- Quarantine workaround documented in README:
  `xattr -dr com.apple.quarantine "/Applications/Easy KONTROL X1 Driver.app"`

## Djay Pro Mapping
Mapping file is included at:
`DJayPro mapping/EASY KONTROL X1 1.djayMidiMapping`

## Files of Interest
- `src/x1_process_hid.rs`: HID read + MIDI output + LED logic
- `src/x1_process.rs`: libusb fallback path
- `src/x1_board.rs` / `board.yml`: control + LED mapping
- `src/menu_bar.rs`: menu bar UI
- `src/main.rs`: app entry + Start/Stop control flag
- `logo/`: menu bar icons + 1024x1024 app icon source
- `logo/Easy KONTROL X1 Driver.icns`: app icon

## Known Warnings
- Many `objc` `cfg` warnings from macros; safe to ignore.
- Some unused fields and variables are benign.

## Release Notes
- v1.1.2: updated app icon, README link points to v1.1.2 release.

## TODO / Future Ideas
- Proper codesigning + notarization for distribution.
- Optional “latest” README release link.
- Improve logging verbosity controls.

