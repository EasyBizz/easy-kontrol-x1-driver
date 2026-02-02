# Easy KONTROL X1 Driver

Easy KONTROL X1 Driver converts Native Instruments KONTROL X1 (mk2) USB events to MIDI on macOS.

## Install (macOS)

### Download (no build)

- Download the latest release zip:
  - https://github.com/EasyBizz/easy-kontrol-x1-driver/releases/tag/v1.1.3
- Unzip it, then move **Easy KONTROL X1 Driver.app** to **/Applications**.

### Build from source

### 1) Build the app

```bash
cargo build --release
```

### 2) Create the .app bundle

```bash
cargo bundle --release
```

This produces:

```
target/release/bundle/osx/Easy KONTROL X1 Driver.app
```

### 3) Install it

- Drag **Easy KONTROL X1 Driver.app** to **/Applications**.

### 4) First launch

- Open **Easy KONTROL X1 Driver** from **Applications**.
- The menu bar icon appears.
- If macOS blocks it, go to **System Settings → Privacy & Security** and click **Open Anyway**.
- If macOS says the app is damaged, run:

```bash
xattr -dr com.apple.quarantine "/Applications/Easy KONTROL X1 Driver.app"
```

### 5) Start/Stop the driver

- Use the menu bar icon to **Start** or **Stop** the driver.
- Connected devices are shown in the menu.

## Mapping

The app creates virtual MIDI ports named **EASY KONTROL X1**.  
Set that port as both input and output in your DJ software.

Djay Pro mapping file: [EASY KONTROL X1 1.djayMidiMapping](DJayPro%20mapping/EASY%20KONTROL%20X1%201.djayMidiMapping)

## AI Handoff

If you want to continue development later (or hand off to another AI), use this file:
[SESSION_HANDOFF.md](AI%20instructions/SESSION_HANDOFF.md)
It contains the current state of the project, key behavior notes, and the main files to look at.

## Development Requirements

- libusb (tested with 1.0.27)
