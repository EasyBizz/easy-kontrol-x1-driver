# Easy KONTROL X1 Driver – DjayPro Mapping Manual

This manual is generated from `DJayPro mapping/EASY KONTROL X1 1.djayMidiMapping`.

**Channels**: Normal actions use MIDI channel 1 (midiChannel 0). Shift actions use MIDI channel 2 (midiChannel 1).  
**FX Hold**: The FX knobs use an alternate mode when holding the matching FX button. That alternate knob mode is sent on MIDI channel 3.

---
## Special

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|

## FX Section (Top)

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| FX1_BUTTON_PLAY | Hold | Deck A FX On/Off | — |
| FX1_BUTTON_1 | Hold | Deck A FX1 On/Off | — |
| FX1_BUTTON_2 | Hold | Deck A FX2 On/Off | — |
| FX1_BUTTON_3 | Hold | Deck A FX3 On/Off | — |
| FX1_KNOB_DRY | Knob | Deck A Filter | **FX Hold (Ch 3):** Deck A FX BPM |
| FX1_KNOB_1 | Knob | Deck A FX1 Dry/Wet | **FX Hold (Ch 3):** Deck A FX1 Parameter |
| FX1_KNOB_2 | Knob | Deck A FX2 Dry/Wet | **FX Hold (Ch 3):** Deck A FX2 Parameter |
| FX1_KNOB_3 | Knob | Deck A FX3 Dry/Wet | **FX Hold (Ch 3):** Deck A FX3 Parameter |
| FX2_BUTTON_PLAY | Hold | Deck B FX On/Off | — |
| FX2_BUTTON_1 | Hold | Deck B FX1 On/Off | — |
| FX2_BUTTON_2 | Hold | Deck B FX2 On/Off | — |
| FX2_BUTTON_3 | Hold | Deck B FX3 On/Off | — |
| FX2_KNOB_DRY | Knob | Deck B Filter | **FX Hold (Ch 3):** Deck B FX BPM |
| FX2_KNOB_1 | Knob | Deck B FX1 Dry/Wet | **FX Hold (Ch 3):** Deck B FX1 Parameter |
| FX2_KNOB_2 | Knob | Deck B FX2 Dry/Wet | **FX Hold (Ch 3):** Deck B FX2 Parameter |
| FX2_KNOB_3 | Knob | Deck B FX3 Dry/Wet | **FX Hold (Ch 3):** Deck B FX3 Parameter |

## Touch Strip

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| STRIP | Knob | Selected Deck: Toggle | turntable2 |

## Deck A

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| DECK_A_ENCODER_BROWSE | Encoder | Library: Scroll | Library: Scroll |
| DECK_A_BUTTON_BROWSE | Hold | Library: Toggle Visible | Library: Toggle Visible |
| DECK_A_BUTTON_LOAD | Hold | Load to Deck A | Library: Back |
| DECK_A_BUTTON_FX1 | Hold | Deck A Instant FX 1 | — |
| DECK_A_BUTTON_FX2 | Hold | Deck A Instant FX 2 | — |
| DECK_A_ENCODER_LOOP | Encoder | Deck A Loop Size | Deck A Loop Move |
| DECK_A_BUTTON_LOOP | Hold | Deck A Loop On/Off | Deck A Loop In/Out |
| DECK_A_BUTTON_CUE | Hold | Deck A Cue (Go to start) | Deck A Set Cue |
| DECK_A_BUTTON_FLUX | Hold | Deck A Tap BPM | — |
| DECK_A_BUTTON_PLAY | Hold | Deck A Play/Pause | — |
| DECK_A_BUTTON_SYNC | Hold | Deck A Sync | — |

## Deck B

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| DECK_B_BUTTON_LOAD | Hold | Load to Deck B | Library: Toggle Source |
| DECK_B_BUTTON_FX1 | Hold | Deck B Instant FX 1 | — |
| DECK_B_BUTTON_FX2 | Hold | Deck B Instant FX 2 | — |
| DECK_B_ENCODER_LOOP | Encoder | Deck B Loop Size | Deck B Loop Move |
| DECK_B_BUTTON_LOOP | Hold | Deck B Loop On/Off | Deck B Loop In/Out |
| DECK_B_BUTTON_CUE | Hold | Deck B Cue (Go to start) | Deck B Set Cue |
| DECK_B_BUTTON_FLUX | Hold | Deck B Tap BPM | — |
| DECK_B_BUTTON_PLAY | Hold | Deck B Play/Pause | — |
| DECK_B_BUTTON_SYNC | Hold | Deck B Sync | — |

## Hotcues

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| DECK_A_HOTCUE_1 | Hold | Deck A Hotcue 1 (jump/play) | Deck A Hotcue 1 (set) |
| DECK_A_HOTCUE_2 | Hold | Deck A Hotcue 2 (jump/play) | Deck A Hotcue 2 (set) |
| DECK_A_HOTCUE_3 | Hold | Deck A Hotcue 3 (jump/play) | Deck A Hotcue 3 (set) |
| DECK_A_HOTCUE_4 | Hold | Deck A Hotcue 4 (jump/play) | Deck A Hotcue 4 (set) |
| DECK_B_HOTCUE_1 | Hold | Deck B Hotcue 1 (jump/play) | Deck B Hotcue 1 (set) |
| DECK_B_HOTCUE_2 | Hold | Deck B Hotcue 2 (jump/play) | Deck B Hotcue 2 (set) |
| DECK_B_HOTCUE_3 | Hold | Deck B Hotcue 3 (jump/play) | Deck B Hotcue 3 (set) |
| DECK_B_HOTCUE_4 | Hold | Deck B Hotcue 4 (jump/play) | Deck B Hotcue 4 (set) |

## Unassigned/Utility

| Control | Type | Normal (Ch 1) | Shift (Ch 2) |
|---|---|---|---|
| CC76 (midiData=76) |  | — | Library (open/focus) |
