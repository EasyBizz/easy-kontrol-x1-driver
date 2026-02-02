use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use midir::os::unix::{VirtualInput, VirtualOutput};
use std::sync::mpsc;

use crate::conf::YamlConfig;
use crate::utils::{hex2bin, knob_to_midi};
use crate::x1_board::{ButtonType, X1mk1Board};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const USB_WRITE_FD: u8 = 0x01;
const LED_DIM: u8 = 0x00;
const LED_BRIGHT: u8 = 0x7F;
const MIDI_CHANNEL: u8 = 0xB0;
const MIDI_CHANNEL_LED: u8 = 0xB2;
const MIDI_CHANNEL_HOTCUE: u8 = 0xB0;
const MIDI_CHANNEL_FX_HOLD: u8 = 0xB2;
const LONG_PRESS_MS: u64 = 800;
const LONG_PRESS_OFFSET: u8 = 32;
const FX_HOLD_MASK_FX1_PLAY: u8 = 1 << 0;
const FX_HOLD_MASK_FX1_1: u8 = 1 << 1;
const FX_HOLD_MASK_FX1_2: u8 = 1 << 2;
const FX_HOLD_MASK_FX1_3: u8 = 1 << 3;
const FX_HOLD_MASK_FX2_PLAY: u8 = 1 << 4;
const FX_HOLD_MASK_FX2_1: u8 = 1 << 5;
const FX_HOLD_MASK_FX2_2: u8 = 1 << 6;
const FX_HOLD_MASK_FX2_3: u8 = 1 << 7;

pub struct X1mk1Hid {
    pub handle: hidapi::HidDevice,
    pub serial_number: String,
    midi_conn_out: MidiOutputConnection,
    midi_conn_in: Option<MidiInputConnection<()>>,
    board: X1mk1Board,
    usb_buffer: [u8; 65],
    usb_timeout: i32,
    led: [u8; 32],
    led_hotcue: [u8; 16],
    led_ext: [u8; 58],
    led_bank80: [u8; 51],
    led_bank81: [u8; 90],
    led_map: std::collections::HashMap<u8, u8>,
    led_report_id: u8,
    led_test: bool,
    led_test_idx: u8,
    led_test_tick: u8,
    led_debug: bool,
    led_all: bool,
    led_clear: bool,
    led_ext_enabled: bool,
    led_bank: u8,
    led_idx: Option<usize>,
    led_once: bool,
    log_byte_changes: bool,
    led_rid: Option<u8>,
    shift_led_idx: Option<u8>,
    shift: u8,
    fx_hold_mask: u8,
    hotcue: bool,
    initialized: bool,  // Flag: true after first stable read
    prev_buf: [u8; 64],
    encoder_quiet_count: u8,
    run_flag: std::sync::Arc<AtomicBool>,
}

impl X1mk1Hid {
    pub fn new(handle: hidapi::HidDevice, serial_number: String, yaml_config: YamlConfig, run_flag: std::sync::Arc<AtomicBool>) -> Self {
        println!("Creating MIDI ports for device");
        const MIDI_PORT_NAME: &str = "EASY KONTROL X1";
        let midi_out = MidiOutput::new("Easy KONTROL X1").unwrap();
        let midi_conn_out = midi_out.create_virtual(MIDI_PORT_NAME).unwrap();
        println!("✓ Created virtual MIDI output port: {}", MIDI_PORT_NAME);
        let board = X1mk1Board::from_yaml(&yaml_config);
        let mut led_map = std::collections::HashMap::new();
        let mut shift_led_idx: Option<u8> = None;
        for (_name, button_type) in &board.buttons {
            match button_type {
                ButtonType::Toggle(b) | ButtonType::Hold(b) | ButtonType::Hotcue(b) => {
                    led_map.insert(b.midi_ctrl_ch, b.write_idx);
                }
                _ => {}
            }
        }
        for (name, button_type) in &board.buttons {
            if name.as_str() == "SHIFT" {
                if let ButtonType::Hold(b) | ButtonType::Toggle(b) | ButtonType::Hotcue(b) = button_type {
                    shift_led_idx = Some(b.write_idx);
                }
            }
        }
        let mut leds = [0x05; 32];
        let led_hotcue = [0x05; 16];
        leds.fill(LED_DIM);
        let usb_buffer = [0; 65];
        let led_test = std::env::var("LED_TEST").ok().as_deref() == Some("1");
        let led_debug = std::env::var("LED_DEBUG").ok().as_deref() == Some("1");
        let led_all = std::env::var("LED_ALL").ok().as_deref() == Some("1");
        let led_clear = std::env::var("LED_CLEAR").ok().as_deref() == Some("1");
        let led_ext_enabled = std::env::var("LED_EXT").ok().as_deref() == Some("1");
        let log_byte_changes = std::env::var("LOG_BYTES").ok().as_deref() == Some("1");
        let led_bank = std::env::var("LED_BANK")
            .ok()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0x80);
        let led_idx = std::env::var("LED_IDX")
            .ok()
            .and_then(|s| s.parse::<usize>().ok());
        let led_once = std::env::var("LED_ONCE").ok().as_deref() == Some("1");
        let led_rid = std::env::var("LED_RID")
            .ok()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .or(Some(0x80));

        Self {
            handle,
            serial_number,
            midi_conn_out,
            midi_conn_in: None,
            board,
            usb_buffer,
            usb_timeout: 50,
            led: leds,
            led_hotcue,
            led_ext: [0; 58],
            led_bank80: [0; 51],
            led_bank81: [0; 90],
            led_map,
            led_report_id: 0,
            led_test,
            led_test_idx: 0,
            led_test_tick: 0,
            led_debug,
            led_all,
            led_clear,
            led_ext_enabled,
            led_idx,
            led_once,
            led_bank,
            log_byte_changes,
            led_rid,
            shift_led_idx,
            shift: 0,
            fx_hold_mask: 0,
            hotcue: false,
            initialized: false,  // Not yet initialized
            prev_buf: [0; 64],
            encoder_quiet_count: 0,
            run_flag,
        }
    }

    fn fx_hold_bit(cc: u8) -> Option<u8> {
        match cc {
            8 => Some(FX_HOLD_MASK_FX1_PLAY),
            10 => Some(FX_HOLD_MASK_FX1_1),
            12 => Some(FX_HOLD_MASK_FX1_2),
            14 => Some(FX_HOLD_MASK_FX1_3),
            9 => Some(FX_HOLD_MASK_FX2_PLAY),
            11 => Some(FX_HOLD_MASK_FX2_1),
            13 => Some(FX_HOLD_MASK_FX2_2),
            15 => Some(FX_HOLD_MASK_FX2_3),
            _ => None,
        }
    }

    fn fx_alt_cc_for_knob(mask: u8, base_cc: u8) -> Option<u8> {
        match base_cc {
            0 if (mask & FX_HOLD_MASK_FX1_PLAY) != 0 => Some(0),
            2 if (mask & FX_HOLD_MASK_FX1_1) != 0 => Some(2),
            4 if (mask & FX_HOLD_MASK_FX1_2) != 0 => Some(4),
            6 if (mask & FX_HOLD_MASK_FX1_3) != 0 => Some(6),
            1 if (mask & FX_HOLD_MASK_FX2_PLAY) != 0 => Some(1),
            3 if (mask & FX_HOLD_MASK_FX2_1) != 0 => Some(3),
            5 if (mask & FX_HOLD_MASK_FX2_2) != 0 => Some(5),
            7 if (mask & FX_HOLD_MASK_FX2_3) != 0 => Some(7),
            _ => None,
        }
    }

    fn set_led_idx(&mut self, idx: u8, val: u8) {
        let i = idx as usize;
        if self.led_bank == 0x80 {
            if i < self.led_bank80.len() {
                self.led_bank80[i] = val;
            }
        } else if self.led_bank == 0x81 {
            if i < self.led_bank81.len() {
                self.led_bank81[i] = val;
            }
        } else if i < self.led.len() {
            self.led[i] = val;
        } else if i < 90 {
            self.led_ext[i - 32] = val;
        }
    }

    fn long_press_cc(&self, short_cc: u8) -> u8 {
        short_cc.saturating_add(LONG_PRESS_OFFSET)
    }

    fn send_press_cc(&mut self, status: u8, cc: u8) {
        let _ = self.midi_conn_out.send(&[status, cc, 127]);
        let _ = self.midi_conn_out.send(&[status, cc, 0]);
    }

    pub(crate) fn init(&mut self, sender: mpsc::Sender<Vec<u8>>) {
        println!("Initializing MIDI input port...");
        const MIDI_PORT_NAME: &str = "EASY KONTROL X1";
        let midi_in = MidiInput::new("Easy KONTROL X1").unwrap();
        let midi_conn_in = midi_in.create_virtual(
            MIDI_PORT_NAME,
            move |_stamp, message: &[u8], _| {
                sender.send(message.to_vec()).unwrap();
            }, ()).unwrap();
        println!("✓ Created virtual MIDI input port: {}", MIDI_PORT_NAME);
        self.midi_conn_in = Some(midi_conn_in);
    }

    pub(crate) fn read(&mut self) -> rusb::Result<()> {
        println!("Reading from device (HID)");
        eprintln!("[HID] report_len=64(+rid) knob_log=v2");
        let (midi_tx, midi_rx) = mpsc::channel::<Vec<u8>>();

        self.init(midi_tx);
        if self.led_debug {
            eprintln!(
                "[LED FLAGS] clear={} all={} idx={:?} once={} ext={} bank=0x{:02x}",
                self.led_clear,
                self.led_all,
                self.led_idx,
                self.led_once,
                self.led_ext_enabled,
                self.led_bank
            );
        }
        if self.led_clear {
            self.led.fill(0);
            self.led_ext.fill(0);
            self.led_bank80.fill(0);
            self.led_bank81.fill(0);
            self.update_leds();
            return Ok(());
        }
        if let Some(idx) = self.led_idx {
            self.led.fill(0);
            self.led_ext.fill(0);
            self.led_bank80.fill(0);
            self.led_bank81.fill(0);
            if self.led_bank == 0x80 {
                if idx < 51 {
                    self.led_bank80[idx] = LED_BRIGHT;
                }
            } else if self.led_bank == 0x81 {
                if idx < 90 {
                    self.led_bank81[idx] = LED_BRIGHT;
                }
            } else {
                if idx < 32 {
                    self.led[idx] = LED_BRIGHT;
                } else if idx < 90 {
                    self.led_ext[idx - 32] = LED_BRIGHT;
                }
            }
            if self.led_debug {
                eprintln!("[LED IDX] set idx={}", idx);
            }
            self.update_leds();
            if self.led_once {
                return Ok(());
            }
        }
        self.update_leds();
        
        let mut read_count = 0;
        loop {
            if !self.run_flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            match midi_rx.try_recv() {
                Ok(message) => {
                    eprintln!("[MIDI IN] {:02x?}", message);
                    if message.len() < 3 {
                        continue;
                    }
                    let status = message[0];
                    let mut i = message[1] as usize;
                    let val = message[2];
                    let mapped = self.led_map.get(&message[1]).copied();
                    if let Some(m) = mapped {
                        i = m as usize;
                    }
                    if let Some(shift_idx) = self.shift_led_idx {
                        if i == shift_idx as usize {
                            // Shift LED is controlled locally, ignore MIDI input.
                            continue;
                        }
                    }
                    let max_idx = if self.led_bank == 0x80 {
                        51
                    } else if self.led_bank == 0x81 {
                        90
                    } else if self.led_ext_enabled {
                        90
                    } else {
                        32
                    };
                    if i >= max_idx {
                        eprintln!("Invalid LED index: {}", i);
                        continue;
                    }
                    match status & 0xF0 {
                        0x90 | 0x80 => {
                            // Note on/off: treat as boolean LED
                            let v = if val == 0 { LED_DIM } else { LED_BRIGHT };
                            self.set_led_idx(i as u8, v);
                        }
                        0xB0 => {
                            // CC: default to LED on/off
                            let v = if val == 0 { LED_DIM } else { LED_BRIGHT };
                            self.set_led_idx(i as u8, v);
                        }
                        _ => {
                            if status == MIDI_CHANNEL_LED {
                                self.set_led_idx(i as u8, val);
                            } else if status == MIDI_CHANNEL_HOTCUE {
                                self.led_hotcue[i] = if val != 0 { LED_BRIGHT } else { LED_DIM };
                            } else {
                                continue;
                            }
                        }
                    }
                    if self.led_debug {
                        eprintln!(
                            "[LED MAP] status=0x{:02x} ctrl=0x{:02x} val={} mapped={:?} -> idx={}",
                            status, message[1], val, mapped, i
                        );
                    }
                    self.update_leds();
                }
                Err(_) => {}
            }
            
            match self.handle.read_timeout(&mut self.usb_buffer, self.usb_timeout) {
                Ok(len) => {
                    if len > 0 {
                        // Zero any unread tail so stale bytes don't linger
                        for i in len..self.usb_buffer.len() {
                            self.usb_buffer[i] = 0;
                        }
                        read_count += 1;
                        let mut buf64 = [0u8; 64];
                        if len >= 65 {
                            // Treat first byte as report ID
                            let report_id = self.usb_buffer[0];
                            if report_id != 0 {
                                eprintln!("[HID] report_id=0x{:02x}", report_id);
                            }
                            buf64.copy_from_slice(&self.usb_buffer[1..65]);
                        } else {
                            // No report ID; take first 64 bytes (or whatever was read)
                            let copy_len = len.min(64);
                            buf64[..copy_len].copy_from_slice(&self.usb_buffer[..copy_len]);
                        }
                        self.read_state(buf64);
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // Only report non-timeout errors
                    if !error_msg.contains("timeout") && !error_msg.contains("Timeout") {
                        eprintln!("HID read error: {}", e);
                        return Err(rusb::Error::Io);
                    }
                }
            }
            if self.led_test {
                self.led_test_tick = self.led_test_tick.wrapping_add(1);
                if self.led_test_tick % 8 == 0 {
                    self.led.fill(LED_DIM);
                    self.led[self.led_test_idx as usize] = LED_BRIGHT;
                    self.led_test_idx = (self.led_test_idx + 1) % 32;
                }
            }
            self.update_leds();
        }
    }

    fn read_state(&mut self, buf: [u8; 64]) {
        // Initialize on first read
        if !self.initialized {
            self.initialized = true;
            
            // Initialize all control states without firing MIDI events
            for (_ctrl_name, button_type) in &mut self.board.buttons {
                match button_type {
                    crate::x1_board::ButtonType::Toggle(ref mut button) => {
                        // Bytes < 8 are noisy with knob/encoder data in HID mode
                        if button.read_i < 8 { continue; }
                        let byte = buf[button.read_i as usize];
                        button.curr = ((byte >> button.read_j) & 1) != 0;
                        button.prev = button.curr;
                    },
                    crate::x1_board::ButtonType::Hold(ref mut button) => {
                        // Bytes < 8 are noisy with knob/encoder data in HID mode
                        if button.read_i < 8 { continue; }
                        let byte = buf[button.read_i as usize];
                        button.curr = ((byte >> button.read_j) & 1) != 0;
                        button.prev = button.curr;
                    },
                    crate::x1_board::ButtonType::Hotcue(ref mut button) => {
                        // Bytes < 8 are noisy with knob/encoder data in HID mode
                        if button.read_i < 8 { continue; }
                        let byte = buf[button.read_i as usize];
                        button.curr = ((byte >> button.read_j) & 1) != 0;
                        button.prev = button.curr;
                    },
                    crate::x1_board::ButtonType::Knob(ref mut k) => {
                        k.curr = knob_to_midi(buf[k.read_i as usize], buf[k.read_j as usize]);
                        k.prev = k.curr;
                    },
                    crate::x1_board::ButtonType::Encoder(ref mut e) => {
                        let mut binnum = [0; 8];
                        hex2bin(buf[e.read_i as usize], &mut binnum);
                        e.curr = match e.read_pos {
                            's' => binnum[0] + binnum[1] * 2 + binnum[2] * 4 + binnum[3] * 8,
                            'n' => binnum[4] + binnum[5] * 2 + binnum[6] * 4 + binnum[7] * 8,
                            _ => 0,
                        };
                        e.prev = e.curr;
                    },
                }
            }
            eprintln!("[INIT] Initialization complete");
            self.prev_buf = buf;
            return; // Skip button processing this read
        }

        // Debug: detect byte changes to discover button layout
        if self.log_byte_changes {
            static mut LAST_BYTES: [u8; 64] = [0; 64];
            static mut FIRST_READ: bool = true;
            unsafe {
                if !FIRST_READ {
                    for i in 0..buf.len() {
                        if buf[i] != LAST_BYTES[i] {
                            eprintln!("[BYTE CHANGE] buf[{}]: 0x{:02x} -> 0x{:02x}", i, LAST_BYTES[i], buf[i]);
                        }
                    }
                }
                LAST_BYTES[..buf.len()].copy_from_slice(&buf);
                FIRST_READ = false;
            }
        }
        
        let mut button_event_bytes = [false; 64];

        let encoder_active = buf[17] != self.prev_buf[17] || buf[18] != self.prev_buf[18];
        if encoder_active {
            self.encoder_quiet_count = 0;
        } else if self.encoder_quiet_count < 3 {
            self.encoder_quiet_count += 1;
        }

        let mut pending_led: Option<(u8, u8)> = None;
        let mut pending_cc: Vec<(u8, u8)> = Vec::new();
        for (ctrl_name, button_type) in &mut self.board.buttons {
            let is_play = matches!(ctrl_name.as_str(), "DECK_A_BUTTON_PLAY" | "DECK_B_BUTTON_PLAY");
            let is_cue = matches!(ctrl_name.as_str(), "DECK_A_BUTTON_CUE" | "DECK_B_BUTTON_CUE");
            let is_play_or_cue = is_play || is_cue;
            let is_shift = ctrl_name.as_str() == "SHIFT";
            let is_hotcue_button =
                ctrl_name.as_str().starts_with("DECK_A_HOTCUE")
                    || ctrl_name.as_str().starts_with("DECK_B_HOTCUE");
            match button_type {
                ButtonType::Toggle(ref mut button) => {
                    if self.hotcue && button.hotcue_ignore {
                        continue;
                    }
                    // Bytes < 8 are noisy with knob/encoder data in HID mode
                    if button.read_i < 8 {
                        continue;
                    }
                    if button.read_i == 23 && self.encoder_quiet_count < 3 {
                        continue;
                    }
                    let byte = buf[button.read_i as usize];
                    let new_state = ((byte >> button.read_j) & 1) != 0;
                    
                    // Per-button debouncing
                    if new_state != button.curr {
                        // Button state changed, reset debounce counter
                        button.curr = new_state;
                        button.debounce_count = 1;
                    } else if button.debounce_count > 0 && button.debounce_count < 3 {
                        // State is consistent, increment counter
                        button.debounce_count += 1;
                    }
                    
                    let debounce_needed = if button.read_i >= 19 { 1 } else { 3 };
                    // Only trigger MIDI on state change if debounced
                    if button.curr != button.prev && button.debounce_count >= debounce_needed {
                        if button.curr {
                            eprintln!("[BUTTON] {} pressed", ctrl_name);
                            button.press_time = Some(Instant::now());
                            if ctrl_name.eq("HOTCUE") {
                                self.hotcue = !self.hotcue;
                                let val = if self.hotcue { LED_BRIGHT } else { LED_DIM };
                                pending_led = Some((button.write_idx, val));
                            }
                            if let Some(bit) = Self::fx_hold_bit(button.midi_ctrl_ch) {
                                self.fx_hold_mask |= bit;
                            }
                            button_event_bytes[button.read_i as usize] = true;
                        } else {
                            if let Some(bit) = Self::fx_hold_bit(button.midi_ctrl_ch) {
                                self.fx_hold_mask &= !bit;
                            }
                            let elapsed = button
                                .press_time
                                .take()
                                .map(|t| t.elapsed().as_millis() as u64)
                                .unwrap_or(0);
                            let is_long = self.shift == 1
                                && !is_play_or_cue
                                && !is_shift
                                && Self::fx_hold_bit(button.midi_ctrl_ch).is_none()
                                && elapsed >= LONG_PRESS_MS;
                            let cc = if is_long {
                                button.midi_ctrl_ch.saturating_add(LONG_PRESS_OFFSET)
                            } else {
                                button.midi_ctrl_ch
                            };
                            let kind = if is_long { "long" } else { "short" };
                            eprintln!("[BUTTON] {} {} -> CC {}", ctrl_name, kind, cc);
                            let status = if is_play || is_shift {
                                MIDI_CHANNEL
                            } else {
                                MIDI_CHANNEL + self.shift
                            };
                            pending_cc.push((status, cc));
                            button_event_bytes[button.read_i as usize] = true;
                        }
                        button.prev = button.curr;
                        button.debounce_count = 0;  // Reset after trigger
                    }
                }
                ButtonType::Hold(ref mut button) => {
                    if self.hotcue && button.hotcue_ignore {
                        continue;
                    }
                    // Bytes < 8 are noisy with knob/encoder data in HID mode
                    if button.read_i < 8 {
                        continue;
                    }
                    if button.read_i == 23 && self.encoder_quiet_count < 3 {
                        continue;
                    }
                    let byte = buf[button.read_i as usize];
                    let new_state = ((byte >> button.read_j) & 1) != 0;
                    
                    // Per-button debouncing
                    if new_state != button.curr {
                        // Button state changed, reset debounce counter
                        button.curr = new_state;
                        button.debounce_count = 1;
                    } else if button.debounce_count > 0 && button.debounce_count < 3 {
                        // State is consistent, increment counter
                        button.debounce_count += 1;
                    }
                    
                    let debounce_needed = if button.read_i >= 19 { 1 } else { 3 };
                    // Only trigger MIDI on state change if debounced
                    if button.curr != button.prev && button.debounce_count >= debounce_needed {
                        if button.curr {
                            eprintln!("[BUTTON] {} pressed", ctrl_name);
                            button.press_time = Some(Instant::now());
                            if is_shift {
                                // Latching shift: toggle on release.
                            }
                            if let Some(bit) = Self::fx_hold_bit(button.midi_ctrl_ch) {
                                self.fx_hold_mask |= bit;
                            }
                            if is_play_or_cue || is_hotcue_button {
                                let status = if is_play {
                                    MIDI_CHANNEL
                                } else {
                                    MIDI_CHANNEL + self.shift
                                };
                                let _ = self.midi_conn_out.send(&[status, button.midi_ctrl_ch, 127]);
                            }
                            button_event_bytes[button.read_i as usize] = true;
                        } else {
                            if let Some(bit) = Self::fx_hold_bit(button.midi_ctrl_ch) {
                                self.fx_hold_mask &= !bit;
                            }
                            let elapsed = button
                                .press_time
                                .take()
                                .map(|t| t.elapsed().as_millis() as u64)
                                .unwrap_or(0);
                            let is_fx_button = Self::fx_hold_bit(button.midi_ctrl_ch).is_some();
                            if is_play_or_cue || is_hotcue_button {
                                let status = if is_play {
                                    MIDI_CHANNEL
                                } else {
                                    MIDI_CHANNEL + self.shift
                                };
                                let _ = self.midi_conn_out.send(&[status, button.midi_ctrl_ch, 0]);
                                button.prev = button.curr;
                                button.debounce_count = 0;
                                continue;
                            }
                            let is_long = self.shift == 1
                                && !is_play_or_cue
                                && !is_shift
                                && !is_fx_button
                                && elapsed >= LONG_PRESS_MS;
                            if is_fx_button && elapsed >= LONG_PRESS_MS {
                                // FX buttons: long hold only affects knob channel, no button action on release.
                                button.prev = button.curr;
                                button.debounce_count = 0;
                                continue;
                            }
                            let cc = if is_long {
                                button.midi_ctrl_ch.saturating_add(LONG_PRESS_OFFSET)
                            } else {
                                button.midi_ctrl_ch
                            };
                            let kind = if is_long { "long" } else { "short" };
                            eprintln!("[BUTTON] {} {} -> CC {}", ctrl_name, kind, cc);
                            if is_shift {
                                self.shift = if self.shift == 0 { 1 } else { 0 };
                                let val = if self.shift == 1 { LED_BRIGHT } else { LED_DIM };
                                pending_led = Some((button.write_idx, val));
                            }
                            let status = if is_play || is_shift {
                                MIDI_CHANNEL
                            } else {
                                MIDI_CHANNEL + self.shift
                            };
                            pending_cc.push((status, cc));
                            button_event_bytes[button.read_i as usize] = true;
                        }
                        button.prev = button.curr;
                        button.debounce_count = 0;  // Reset after trigger
                    }
                }
                ButtonType::Hotcue(ref mut button) => {
                    if !self.hotcue {
                        continue;
                    }
                    let byte = buf[button.read_i as usize];
                    if button.read_i < 8 {
                        continue;
                    }
                    if button.read_i == 23 && self.encoder_quiet_count < 3 {
                        continue;
                    }
                    button.curr = ((byte >> button.read_j) & 1) != 0;
                    if button.curr == button.prev {
                        continue;
                    } else if button.curr {
                        button.press_time = Some(Instant::now());
                        button_event_bytes[button.read_i as usize] = true;
                        button.prev = button.curr;
                    } else {
                        let elapsed = button
                            .press_time
                            .take()
                            .map(|t| t.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        let is_long = self.shift == 1
                            && !is_play_or_cue
                            && !is_shift
                            && Self::fx_hold_bit(button.midi_ctrl_ch).is_none()
                            && elapsed >= LONG_PRESS_MS;
                        let cc = if is_long {
                            button.midi_ctrl_ch.saturating_add(LONG_PRESS_OFFSET)
                        } else {
                            button.midi_ctrl_ch
                        };
                        let kind = if is_long { "long" } else { "short" };
                        eprintln!("[BUTTON] {} {} -> CC {}", ctrl_name, kind, cc);
                        let status = MIDI_CHANNEL + self.shift;
                        pending_cc.push((status, cc));
                        button_event_bytes[button.read_i as usize] = true;
                        button.prev = button.curr;
                    }
                }
                ButtonType::Knob(_) | ButtonType::Encoder(_) => {
                    // Knobs/encoders are handled in the second pass
                }
            }
        }

        let fx_hold_mask = self.fx_hold_mask;
        let shift_active = self.shift;
        for (_ctrl_name, button_type) in &mut self.board.buttons {
            match button_type {
                ButtonType::Knob(ref mut k) => {
                    let raw_i = buf[k.read_i as usize];
                    let raw_j = buf[k.read_j as usize];
                    let new_val = knob_to_midi(raw_i, raw_j);
                    if new_val != k.curr {
                        eprintln!(
                            "[KNOB] read_i={} read_j={} raw=0x{:02x}/0x{:02x}: {} -> {}",
                            k.read_i,
                            k.read_j,
                            raw_i,
                            raw_j,
                            k.curr,
                            new_val
                        );
                    }
                    k.curr = new_val;
                    if k.curr != k.prev {
                        // Skip MIDI if this knob shares a byte with a button event this frame
                        if button_event_bytes[k.read_i as usize] || button_event_bytes[k.read_j as usize] {
                            k.prev = k.curr;
                            continue;
                        }
                        let cc = k.midi_ctrl_ch;
                        let status = if Self::fx_alt_cc_for_knob(fx_hold_mask, cc).is_some() {
                            MIDI_CHANNEL_FX_HOLD
                        } else {
                            MIDI_CHANNEL + shift_active
                        };
                        let _ = self.midi_conn_out.send(&[status, cc, k.curr]);
                    }
                    k.prev = k.curr;
                }
                ButtonType::Encoder(ref mut encoder) => {
                    let mut binnum = [0; 8];
                    hex2bin(buf[encoder.read_i as usize], &mut binnum);
                    match encoder.read_pos {
                        's' => {
                            encoder.curr = binnum[0] + binnum[1] * 2 + binnum[2] * 4 + binnum[3] * 8;
                        }
                        'e' => {
                            encoder.curr = binnum[4] + binnum[5] * 2 + binnum[6] * 4 + binnum[7] * 8;
                        }
                        _ => panic!("Invalid read_pos"),
                    }
                    if encoder.curr != encoder.prev {
                        // Clockwise init
                        let mut velocity = 1;
                        if (encoder.prev == 15 && encoder.curr == 0) || (encoder.prev == 0 && encoder.curr == 15) {
                            // Full rotation special case
                            velocity = if encoder.prev == 15 { 1 } else { 127 };
                        } else if encoder.curr > encoder.prev {
                            velocity = 1;
                        } else {
                            velocity = 127;
                        }
                        let _ = self.midi_conn_out.send(&[MIDI_CHANNEL + shift_active, encoder.midi_ctrl_ch, velocity]);
                    }
                    encoder.prev = encoder.curr;
                }
                ButtonType::Toggle(_) | ButtonType::Hold(_) | ButtonType::Hotcue(_) => {}
            }
        }
        if let Some((idx, val)) = pending_led {
            self.set_led_idx(idx, val);
        }
        for (status, cc) in pending_cc {
            self.send_press_cc(status, cc);
        }
        self.prev_buf = buf;
    }

    fn update_leds(&mut self) {
        let mut led = self.led;
        if self.hotcue {
            for i in 9..25 {
                led[i] = self.led_hotcue[i - 9];
            }
        }
        if self.led_debug {
            let preview_len = 8.min(led.len());
            eprintln!("[LED STATE] {:?}", &led[..preview_len]);
        }
        let mut payloads: Vec<(Vec<u8>, &'static str)> = Vec::new();
        let fill_all = if self.led_clear {
            Some(0)
        } else if self.led_all {
            Some(LED_BRIGHT)
        } else {
            None
        };
        let mut rid_filter = self.led_rid;
        if self.led_clear {
            // On clear, hit all known output reports.
            rid_filter = None;
        }

        // Raw 32-byte payload (no report ID)
        if rid_filter.is_none() {
            let mut raw = led.to_vec();
            if let Some(v) = fill_all {
                raw.fill(v);
            }
            payloads.push((raw, "write:len32"));
            // 33-byte payloads with report IDs 0 and 1
            let mut buf0 = vec![0u8; 33];
            buf0[0] = 0;
            if let Some(v) = fill_all {
                for b in &mut buf0[1..] { *b = v; }
            } else {
                buf0[1..].copy_from_slice(&led);
            }
            payloads.push((buf0, "write:rid0_len33"));
            let mut buf1 = vec![0u8; 33];
            buf1[0] = 1;
            if let Some(v) = fill_all {
                for b in &mut buf1[1..] { *b = v; }
            } else {
                buf1[1..].copy_from_slice(&led);
            }
            payloads.push((buf1, "write:rid1_len33"));
        }

        // HID output reports for X1 MK2: report IDs 0x80 (51 bytes) and 0x81 (90 bytes)
        if rid_filter.is_none() || rid_filter == Some(0x80) {
            let mut buf80 = vec![0u8; 52]; // 1 + 51
            buf80[0] = 0x80;
            if let Some(v) = fill_all {
                for b in &mut buf80[1..] { *b = v; }
            } else {
                if self.led_bank == 0x80 {
                    buf80[1..].copy_from_slice(&self.led_bank80);
                } else {
                    buf80[1..1 + led.len()].copy_from_slice(&led);
                }
            }
            payloads.push((buf80, "write:rid80_len52"));
        }
        if (self.led_ext_enabled && (rid_filter.is_none() || rid_filter == Some(0x81)))
            || rid_filter == Some(0x81)
        {
            let mut buf81 = vec![0u8; 91]; // 1 + 90
            buf81[0] = 0x81;
            if let Some(v) = fill_all {
                for b in &mut buf81[1..] { *b = v; }
            } else {
                if self.led_bank == 0x81 {
                    buf81[1..].copy_from_slice(&self.led_bank81);
                } else {
                    buf81[1..1 + led.len()].copy_from_slice(&led);
                    if self.led_ext_enabled {
                        buf81[1 + led.len()..].copy_from_slice(&self.led_ext);
                    }
                }
            }
            payloads.push((buf81, "write:rid81_len91"));
        }

        let mut ok = false;
        for (payload, label) in payloads.iter() {
            match self.handle.write(payload) {
                Ok(n) => {
                    ok = true;
                    if self.led_debug {
                        eprintln!("[LED WRITE] ok {} len={} wrote={}", label, payload.len(), n);
                    }
                }
                Err(e) => {
                    if self.led_debug {
                        eprintln!("[LED WRITE] err {} len={} err={}", label, payload.len(), e);
                    }
                }
            }
        }

        // Try feature reports as a fallback (some HID devices require this)
        if self.led_clear {
            // Known feature report IDs from descriptor: d0/d1/d2/d8/d9 (32 bytes), f0 (8 bytes), f1 (16 bytes)
            for rid in [0xD0u8, 0xD1, 0xD2, 0xD8, 0xD9] {
                let mut f = vec![0u8; 33];
                f[0] = rid;
                payloads.push((f, "feat:dX_len33"));
            }
            let mut f0 = vec![0u8; 9];
            f0[0] = 0xF0;
            payloads.push((f0, "feat:f0_len9"));
            let mut f1 = vec![0u8; 17];
            f1[0] = 0xF1;
            payloads.push((f1, "feat:f1_len17"));
        }
        for (payload, label) in payloads.iter() {
            match self.handle.send_feature_report(payload) {
                Ok(()) => {
                    ok = true;
                    if self.led_debug {
                        eprintln!("[LED FEAT] ok {} len={}", label, payload.len());
                    }
                }
                Err(e) => {
                    if self.led_debug {
                        eprintln!("[LED FEAT] err {} len={} err={}", label, payload.len(), e);
                    }
                }
            }
        }

        if !ok && self.led_report_id == 0 {
            self.led_report_id = 1;
        }
    }
}
