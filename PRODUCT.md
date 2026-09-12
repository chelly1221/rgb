# RGB Switch

<!-- impeccable:product-schema 1 -->

## Platform
web

Windows desktop app in a Tauri WebView with a custom draggable titlebar and minimize/maximize/close controls. Default client 760 × 760; minimum 520 × 560.

## Stack
User approved Tauri 2 + React + TypeScript and code-first implementation. Final engine: direct Rust device modules without an OpenRGB runtime/server.

## Product Purpose
A small local Windows EXE for whole-group and individual RGB ON/OFF, static colors and native animations on the owner's PC.

## Users
One PC owner. Hardware is discovered locally. Korean UI, no account or cloud.

## Capabilities and Constraints
Five controller groups support direct ON/OFF: HARPOON wired HID, Palit RTX 3070 NVAPI/I²C, MSI 7E40 global switch, GS3087T wired feature reports, and two CORSAIR DDR5 modules over SMBus. RAM uses administrator rights and the exact driver files embedded in the personal EXE. Board-connected fans follow the board global switch; physical wiring needs observation. Whole-group commands target supported devices. GPU/board/keyboard/RAM offer static, breathing and spectrum effects with color, brightness and speed; successful settings are remembered during this run and reused by ON. Mouse ON uses onboard lighting and preserves DPI stage colors. OFF affects lighting only. Last-command state is not optical telemetry. Other lighting software may overwrite settings. Unknown hardware is rejected.

## Open Decisions
RGB Switch is a working name. Controller-level OFF/ON verification is complete for five groups. Optical confirmation and fan wiring remain distinct from protocol readback. The user did not observe the latest fan test. The user did confirm DPI indicator colors change with stages after the mouse fix.
