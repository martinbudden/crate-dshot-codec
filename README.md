# `dshot-codec` Rust Crate<br>![license](https://img.shields.io/badge/license-MIT-green) [![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0) ![open source](https://badgen.net/badge/open/source/blue?icon=github)

`dshot-codec` supports the encoding and decoding of `Dshot` data.

`dshot-codec` implements the hardware-independent part of `Dshot`, it does no hardware manipulation itself. Rather it
provides a foundation that can be used to write `Dshot` device drivers.

More specifically `dshot-codec` supports the encoding of commands sent from the Flight Controller(FC) to the ESC (Electronic Speed Controller),
and the decoding of telemetry sent from the ESC to the FC.

It has a number of `struct`s and `enum`s to do this:

|                       | Purpose                                     | Mode                            |
| :-------------------- | :------------------------------------------ |:------------------------------- |
| `DshotCommandFrame`   | Send an RPM value or a command to the ESC   | Unidirectional or Bidirectional |
| `DshotSpeed`          | `enum`, protocol speed                      | Unidirectional or Bidirectional |
| `DshotCommand`        | `enum` of commands available (eg `Beep1`)   | Unidirectional or Bidirectional |
| `DshotError`          | `enum`, Error handling                      | Unidirectional or Bidirectional |
|                       |                                             |                                 |
| `DshotTelemetryFrame` | Decoded data received from the ESC          | Bidirectional                   |
| `TelemetryType`       | `enum` type of telemetry requested/received | Bidirectional                   |
| `GcrFrame`            | Used in decoding ESC telemetry              | Bidirectional                   |
| `NrziFrame`           | Used in decoding ESC telemetry              | Bidirectional                   |

## Sending a command

`DshotCommandFrame` is used to send a command (ie set the motor eRPM, or Beep) from the FC to the ESC.

It is used in both unidirectional and bidirectional mode (see the table below to see how this is achieved).

### Unidirectional vs Bidirectional Dshot Modes

| Operational Aspect     | Unidirectional (Throttle)           | Unidirectional (Commands)                 | Bidirectional (Throttle & Commands)                |
| :--------------------- | :---------------------------------- | :---------------------------------------- | :------------------------------------------------- |
| **Telemetry Bit**      | **`false`** (Set to `0`)            | **`true`** (Set to `1`)                   | **`true`** (Set to `1`)                            |
| **XOR Checksum Mode**  | **Standard**                        | **Bitwise Inverted**                      | **Bitwise Inverted**                               |
| **ESC Action**         | Executes throttle<br>Remains silent | Executes command<br>Returns a ghost reply | Executes command<br>Returns a telemetry frame      |
| **FC Pin Mode**        | Permanent **Output**                | Permanent **Output**                      | Flips from **Output to Input** right after TX      |
| **Repetition Gate**    | Streams continuously                | **Must repeat ~10 times** to execute      | Commands **must repeat ~10 times** to execute      |
| **FC Software Action** | Fire-and-forget stream              | Fire-and-forget stream                    | Transmits, then pauses ~30µs to capture `GcrFrame` |

## Decoding telemetry data from the ESC

The ESC sends data in an **NRZI** encoded bitstream.

On STM32 microcontrollers this data is captured in a 21-bit `NrziFrame` which is decoded to a 20-bit `GcrFrame`.

On Raspberry Pi Pico microcontrollers **PIO** is used to capture this data directly in a `GcrFrame`.

This `GcrFrame` is then decoded to a `DshotTelemetryFrame` which can then be directly used by the host software.

`dshot-codec` contains methods for decoding `NrziFrame`s and `GcrFrames`.

The process is illustrated below:

### STM32 microcontrollers

```text
[ Microcontroller Pin via Input Capture ]
                    │
                    ▼
             [ 21-bit NRZI ]
                    │  (Strip leading zero, decode NRZI transitions)
                    ▼
             [ 20-bit GCR ]
                    │  (Split into 4x 5-bit chunks, apply GCR lookup)
                    ▼
       [ 16-bit DshotTelemetryFrame ]
```

### Raspberry Pi microcontrollers

These use PIO to capture the pin transitions directly as GCR

```text
[ Microcontroller Pin via PIO ]
                    │
                    ▼
             [ 20-bit GCR ]
                    │  (Split into 4x 5-bit chunks, apply GCR lookup)
                    ▼
        [ 16-bit DshotTelemetryFrame ]
```

## **NRZI** frames

**NRZI** stands for Non-Return-to-Zero, Inverted.

It is a method of mapping digital binary bits (0s and 1s) into physical voltage changes on a wire.
In standard digital communication (like normal `Dshot` commands), a high voltage represents a 1 and a low voltage represents a 0.
This is known as standard **NRZ** (Non-Return-to-Zero).

**NRZI** works differently by focusing on the transitions (edges) rather than the absolute voltage levels:

* A 1 bit forces the signal wire to change state (if it was High, it flips to Low; if it was Low, it flips to High).
* A 0 bit forces the signal wire to stay the same (no change in voltage level).

## Why `DShot` Telemetry uses **GCR** + **NRZI**

Microcontrollers read incoming data by measuring the time between voltage transitions.

If an ESC sent a long string of 0 bits over normal wiring, the voltage line would just sit perfectly flat for a long time.
The microcontroller's internal clock would lose synchronization, and incorrectly read the incoming data packet.

By combining `GCR` and `NRZI`, the `DShot` protocol ensures synchronization:

* GCR ensures that there are never have more than two 0 bits in a row in hte data stream.
* Because there are mostly 1 bits, `NRZI` forces the physical wire to constantly flip back and forth between `HIGH` and `LOW`.

These constant flips act like a heartbeat, keeping the microcontroller's input capture timers synchronized with the ESC's transmission clock.

## Capture Mechanism

| Architecture    | Primary Hardware Peripheral | Raw Data Form in RAM                                                                   |
| --------------- | --------------------------- | -------------------------------------------------------------------------------------- |
| RP2040 / RP2350 | PIO + DMA                   | `u32` containing raw bit values                                                        |
| STM32           | Timer Input Capture + DMA   | `[u32; 21]` array of clock timestamps                                                  |
| ESP32           | RMT                         | Array of `RmtPulse` elements specifying the microsecond duration of each high/low peak |

## Dshot specification

The [Dshot](https://blck.mn/2016/11/dshot-the-new-kid-on-the-block/) protocol
is based on [W2812B](https://cdn-shop.adafruit.com/datasheets/WS2812B.pdf) (`NeoPixel`) protocol.

See also: [DSHOT - the missing Handbook](https://brushlesswhoop.com/dshot-and-bidirectional-dshot/).

See <https://en.wikipedia.org/wiki/Run-length_limited#GCR:_(0,2)_RLL> for details of the GCR encoding.

### Variants

| Protocol | Effective Baud Rate | Frame Duration | Max Theoretical Refresh Rate |
| -------- | ------------------- | -------------- | ---------------------------- |
| Dshot150 |            150 Kbps |      106.7 μ s |                     9.37 kHz |
| Dshot300 |            300 Kbps |       53.3 μ s |                    18.75 kHz |
| Dshot600 |            600 Kbps |       26.7 μ s |                    37.50 kHz |

`Dshot150` means 150 kilobytes/second, `Dshot300` means 300 kilobytes/second

* `T0` is the width of the pulse
* `T1` is the width of gap to the next pulse

`WS2812B` specification is

```text
    T0H = 400ns +/- 150ns
    T1H = 800ns +/- 150ns
    T0L = 850ns +/- 150ns
    T1L = 450ns +/- 150ns
    TxH+TxL = 1250ns +/- 600ns (T0H + T0L or T1H + T1L)
    W2818B_T0H = 400
    W2818B_T1H = 800
    W2818B_T = 1250
```

`Dshot150` specification is

```text
    T0H = 2500ns (data low pulse width)
    T0L = 4180ns (data low gap width)
    T1H = 5000ns (data high pulse width)
    T1L = 1680ns (data high gap width)
    TxH+TxL = 6680ns  (T0H + T0L or T1H + T1L)
```

`Dshot300` specification is

```text
    T0H = 1250ns (data low pulse width)
    T0L = 2090ns (data low gap width)
    T1H = 2500ns (data high pulse width)
    T1L =  840ns (data high gap width)
    TxH+TxL = 3340ns  (T0H + T0L or T1H + T1L)
```

`Dshot600` specification is

```text
    T0H =  625ns (data low pulse width)
    T0L = 1045ns (data low gap width)
    T1H = 1250ns (data high pulse width)
    T0L =  420ns (data hig gap width)
    TxH+TxL = 1670ns  (T0H + T0L or T1H + T1L)
```

### `no_std`

This crate is `no_std`, that it does not link to the standard library and so does not depend on an operating system
and uses no allocation. This means it is suitable for embedded system.

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
