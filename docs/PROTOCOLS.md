# Direct device modules

These narrowly scoped Rust adapters do not launch, link or communicate with OpenRGB. Public controller implementations were consulted as interoperability references; no OpenRGB source files are vendored.

## HARPOON wired USB

`devices/corsair.rs` opens only 1B1C:1B5E interface 1 / usage FF42:0001 and checks the product property. A 65-byte Windows output report contains report ID 0, address 8, command/property/options. The 64-byte reply starts with 00, command, status. Lighting switches to volatile software mode, closes any stale transaction, opens an endpoint, writes six planar RGB bytes and closes it. This mouse accepts endpoint 1, selected after explicit rejection of endpoint 0x22. Responses and timeouts are checked. Failure attempts previous render-mode restoration.

Actual mouse OFF and white ON were acknowledged. Physical LED output remains unconfirmed. Wireless routing and original effects restoration are not implemented.

References: [controller](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/CorsairPeripheralV2Controller/CorsairPeripheralV2Controller.cpp), [lighting transactions](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/CorsairPeripheralV2Controller/CorsairPeripheralV2SoftwareController.cpp).

## Palit RTX 3070

`devices/gpu.rs` loads the installed NVIDIA driver's system `nvapi64.dll`. Device ID 248410DE and subsystem 24841569 must both match. The adapter checks the I²C controller signature and mode at address 0x49, port 1. Registers 6C/6D/6E contain RGB, 6F brightness, E0 mode and 60 control. OFF clears mode/control; white ON sets RGB/brightness FF and control 1. All six values are checked by readback. Failed writes attempt snapshot rollback.

Actual results: OFF readback OK; ON readback OK; original six-register snapshot restored successfully. Register verification is not an optical measurement. Private NVAPI I²C entry points may need adaptation after driver changes. GPU fan speed and other GPU settings are outside this module.

References: [Palit detection](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/PalitGPUController/PalitGPUControllerDetect.cpp), [Palit v1](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/PalitGPUController/PalitGPUv1Controller.cpp), [NVAPI transport](https://github.com/CalcProgrammer1/OpenRGB/blob/master/i2c_smbus/Windows/i2c_smbus_nvapi.cpp).

## MSI 7E40 / MB800

The exact installed Mystic Light `MSI_800sLed` implementation supplied the packet layout. The inspected `MysticLight_AllDevice.dll` SHA-256 is `9D244614371C0DCC7D0BA90F35C40ECA3851BEF5E9CF3259FE77CE82153E1341`. Runtime uses only our HID adapter, not this vendor DLL. Board identity, HID serial prefix 7E40, firmware 2.0 and the 290-byte feature report 0x50 are checked. The shared `Global\\Access_USB_Sensors` mutex coordinates with MSI software.

64-byte output reports use ID 1. B0 reads firmware, BA reads the global switch, BB sets byte 6 to 0/1. Replies begin 01/5A. Power changes require ACK and BA readback; failure attempts rollback. Actual OFF/ON passed and all 290 effect bytes stayed unchanged. This controls the board's global lighting gate and attached headers, not fan motors. Header wiring is not software-enumerable; the user missed the five-second optical test.

Reference: [MSI detection families](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/MSIMotherboardController/MSIMotherboardControllerDetect.cpp). Exact packet evidence comes from the locally installed MSI binary and actual device responses.

## GANSS GS3087T

The [official GANSS downloads page](https://www.ganss.cn/page/drives/) links the GS3104T/GS3087T/GS104C/GS87C driver archive dated 20240923. The extracted exact-model `device.xml`, native `DeviceDriver.exe` 1.1.0.7 disassembly, and captured HID calls supplied the verified lighting format. These files remain in local temporary research storage, outside the application.

The correct RGB feature endpoint is interface 0 / usage 0001:0006 with a 95-byte descriptor. The vendor-specific interface 1 is input-only. Windows feature buffers are 65 bytes: report ID 0 plus 64 bytes. Requests use 04/opcode; handshake 18, lighting 13 with one block, raw effect data, and commit 02. ACK must contain the matching opcode and status 1. A firmware FF response permits one bounded resend, as observed in the official driver.

The raw lighting payload uses effect 0=OFF, 1=static; RGB bytes 2–4 are FF; brightness byte 10 is 16; speed byte 11 is 11; **byte 12 repeats the active effect ID**. Bytes 15/16 are AA/55. The earlier test omitted byte 12 and stayed pink, as the user reported. The corrected module waits 150 ms after commit and queries F5: nine blocks of sixteen indexed RGB slots, then closes the query with 02. Verified OFF was 144 zero slots; verified white ON was 88 FD/FD/FD slots and 56 zero slots. The F0 flash-save command is omitted. Key maps and firmware writes are not exposed.

The manufacturer app continues polling after its window closes to the tray. The analysis-launched process and capture helper were terminated before final verification. Concurrent vendor control may produce stale replies, which the adapter rejects.

## CORSAIR VENGEANCE RGB DDR5 pair

This PC has two CMH128GX5M2B6400C42 modules at SMBus 19/1B. Identity queries establish VID/PID 1B1C:0901, protocol 4, firmware 0.5.6; info and effect reads verify CRC-8 polynomial 07. Runtime validates the board, both DIMM part numbers, Intel 8086:7F23's OS-assigned port resource 4000–401F, and the exact installed `CorsairLLAccessLib64.dll` SHA-256 `DE834E260B50BCB00063618004EE664DC7117BEA90E03C1FA04C2F619DAAC29E` before loading it.

The signed installed DLL exports a function table; only existing-driver open/close and byte-port I/O entries are used. Administrator rights and the already-installed Corsair driver are required. No driver install/uninstall entry, physical-memory function, PCI write, or frontend-supplied address is exposed. Both `Global\\Access_SMBUS` and `Global\\Access_SMBUS.HTP.Method` are held with bounded waits. SMBus byte-data transfers use only this controller's status, command, address and data registers; an active bus or timeout stops without forced reset.

Info buffer selection is 61=0; effect read selection 63=1; cursor reset 21=0; bytes read from 40 with checksum 42. A 20-byte effect is staged through 0B=0, 21=0, repeated 20 writes, checked against 42, then applied with 82=1. This firmware temporarily NACKs while applying; bounded status-30 polling handles that interval, and full effect readback must match before success. OFF changes only brightness bytes 7/11 to zero. ON changes those bytes to FF, retaining the effect. Failure attempts restoration of both prior effects and reports restoration errors explicitly.

The first test failed during the immediate post-apply status read, not during effect staging. After fixing the bounded wait, both modules passed OFF/ON and restoration to the original rainbow effect. No SPD (50–57), bootloader, firmware, or persistent-save command is permitted by the adapter.

Interoperability references: [Corsair DRAM register definitions](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/CorsairDRAMController/CorsairDRAMController.h), [effect format](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/CorsairDRAMController/CorsairDRAMController.cpp), [Intel i801 host register behavior](https://github.com/torvalds/linux/blob/master/drivers/i2c/busses/i2c-i801.c). No third-party controller source is vendored or linked. Register facts were implemented in the local Rust adapter and validated on the actual kit.

## Validation

Eleven Rust unit tests cover model gates, request validation, packet encoding including the keyboard active-effect byte, response errors, RAM register restrictions and CRC, and the x64 NVAPI ABI size. Tests do not access hardware. TypeScript compilation, Clippy with warnings denied, and release EXE compilation passed. Five controller groups are implemented; RAM availability depends on elevation and its verified installed driver. Hardware diagnostics were run separately with device readback. The app serializes scans/writes and reports per-device failures. No service, startup task or automatic lighting change is installed.

## 2026-09-12: native lighting and embedded transport update (current implementation)

This section supersedes the earlier white-only mouse behavior and installed-only memory driver requirements above.

- **HARPOON:** no software RGB frame is sent anymore. GET brightness property 02 returns a 16-bit value in 0..1000. Brightness writes require software mode 2; SET 02=0/1000 is followed by SET mode 3=1 (hardware). Both brightness and mode are reread, and DPI_INDEX 1E is checked unchanged. On error, previous brightness/mode restoration is attempted. No DPI value, stage color, key assignment or flash command is written. Actual OFF/ON passed; mode 1 and DPI stage were preserved. The user confirmed DPI-button presses change the indicator color again. Reference: [ckb-next Bragi properties](https://github.com/ckb-next/ckb-next/blob/master/src/daemon/bragi_proto.h), [hardware brightness](https://github.com/ckb-next/ckb-next/blob/master/src/daemon/led_bragi.c). Custom mouse colors/effects are intentionally unavailable while retaining onboard DPI behavior.
- **Palit v2:** static, native breathing and spectrum use only lighting registers 6C..72, 62..65, E1..E3, E0,60 (exact list in source). Primary RGB/brightness precede mode/control. Breathing uses control 22 and 16-bit periods 2000/1000/500; cycle uses E0=1, E1=0, E2=80/40/20, E3=1F. Full 16-register snapshots, readback and failure rollback. All three requests and original restoration passed. Reference: [Palit v2 controller](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/PalitGPUController/PalitGPUv2Controller.cpp).
- **MSI MB800:** feature 50 has 18 × 16-byte areas and byte 289 Store. Actual 7E40 FW2.0 accepts areas 0,1,2,3,9,17 (JARGB1/2/3, JAF, JRGB1, SelectAll); absent areas read zero. Modify only these six, preserve cycle count and reserved option bits, keep Store=0. Modes 2 steady, 4 breathing, 5 color ring; Option2 has user-color bit5, brightness 0..5 in bits2..4, speed0..2 in bits0..1. Read all 289 non-store bytes back exactly and restore settings/global switch on failure. Initial all-area write failed readback and restored; restricting to present areas made all three effects and restoration pass. Fan wiring remains physically unconfirmed.
- **GS3087T:** exact-model device.xml modes 1 static, 7 breathing, 8 spectrum. Both mode bytes 1 and12 match. RGB in2..4, brightness round(percent×15/100)+1 at10, speed4/9/16 at11; F0 remains omitted. Commands ACKed. At 60% with RGB FF/5A/8C, static read 88 lit slots at 158/56/87; breathing and spectrum returned varying rendered colors. Dynamic slot queries span several frames and do not assert instantaneous equality. Static/animation method verifies the handshake and commit ACK; the hardware diagnostic separately inspected rendered frames.
- **DDR5:** 20-byte effects now expose mode10 static,1 pulse,8 rainbow; speed0..2, custom-color selector, paired RGB and brightness bytes7/11. Existing CRC, bounded apply polling, exact readback and two-DIMM rollback remain. All three modes passed CRC/readback and restore on both modules.
- **Embedded personal driver:** DLL SHA DE834E260B50BCB00063618004EE664DC7117BEA90E03C1FA04C2F619DAAC29E, SYS SHA 020ACBF028C67C0936798AD7BA8436D0D58603912F5019A5E56898C2211CF056. Original valid Corsair DLL signature and Microsoft Windows Hardware Compatibility Publisher SYS signature inspected. `include_bytes!` embeds both unchanged plus EULA; private inputs excluded from version control. Protected Program Files extraction, hash verification and read handles denying write/delete sharing guard the load. The pinned DLL table[3] RVA BA80 identifies the image; its initialized service-name UTF-16 buffer at RVA4C860 is inspected only. Own service CorsairLLAccessB71D15DD68159BC5C809912FC25EE2B7F38391B4 points only to the extracted SYS and is SERVICE_KERNEL_DRIVER, DEMAND_START. SCM adds an NT `\??\` prefix, normalized only for the exact expected path comparison. Start waits at most five seconds. Existing iCUE services are not modified; vendor installer/uninstaller exports are not called. The main EXE requests administrator rights in its manifest. Driver may remain loaded until shutdown; no auto-start service or startup task is created.
- **Diagnostic fix:** hidden Windows PowerShell had inherited PowerShell7 PSModulePath, making Get-FileHash unavailable. Child now removes PSModulePath, explicitly imports Windows CimCmdlets, and returns fixed ASCII stage codes. Binary hashes are verified directly in Rust. Embedded-path RAM read probe passed independently of iCUE's DLL path.
- **App:** strict typed Static/Breathe/Spectrum settings, RGB bytes, brightness0..100, speed1..3, fixed ID/key gates; hardware access serialized with existing session mutex. Native effects need no animation worker. Successful settings are kept only for this app session; OFF then ON reapplies them. No automatic startup application. UI shows command outcomes, never claims optical measurement.

19 Rust tests cover protocol boundaries, absent MSI areas, no-flash flags, effect encoding, private driver hashes and service paths. Separate physical tests passed as described above. Selected flame icon and bundled Pretendard remain unchanged. Proprietary driver files are personal build inputs; redistribution rights were not established.

## Fan OFF investigation: readback is not optical confirmation

The user reported all case fans remaining lit after global OFF, static black,
static black while the two MSI lighting services were temporarily stopped,
independent white with native brightness B0, and native Off with the global
controller enabled. The last case remained green despite all six present areas
reading mode 0 and black colors. Both MSI lighting services and LEDKeeper2 were
restored. Blue and delayed green were observed, but a later MSI UI request for
pink was reported as blue; these color changes do not establish a causal match
to the requested color. Physical fan OFF is unresolved.

A bounded Frida observation of the existing LEDKeeper2 process filtered HID
traffic to USB 0DB0:0076. MSI Center's Steady Apply sent feature 50 twice, with
Store=0 then Store=1. Four ARGB areas retained their profile settings and cycle
count 30. JRGB1 and SelectAll transmitted cycle count **255**, whereas a subsequent
GetFeature returned **0** at both offsets 160 and 288. SelectAll held mode 2,
RGB FF/2E/60, option B5. MSI UI Off then sent SelectAll mode 0, again with both
cycle fields 255. No extra 64-byte output command was captured for these applies.

`outgoing_settings` now supplies 255 for those two outbound fields and still
forces Store=0. Verification requires their observed readback value 0 and exact
equality for every other non-store byte; it does not ignore arbitrary mismatches.
A regression test covers this asymmetry and rejects unrelated mode, color,
brightness and ARGB count changes. All 20 unit tests and Clippy pass. The change
corrects an observed protocol discrepancy; it is not yet a verified fan fix.

`fan_hold status` only reads, and diagnostic result files include command, start
timestamp, settings and global state. `black`, `dim-local` and `vendor-off` hold
their setting without a timed restore, allowing an unambiguous user observation.
`restore` uses the original snapshot saved before the first hold. Diagnostic
traffic capture detaches automatically after 180 seconds and does not change
MSI service configuration.

## Fan OFF resolution: explicit Gen1 port initialization

This supersedes the unresolved fan findings above. Windows SMBIOS exposes the
motherboard model but no case model. Present USB/HID devices include no separate
fan controller. MSI Case's current log reports failed device initialization, and
Cube Fan reports EX_Support=false / IsConnected=99; neither identifies a fan model.

MSI Center's JARGB_V2_1 Scan was observed directly. It sent three output commands
`01 82 ... port=0` about one second apart and then `01 84 ... port=0 enable=0`.
The UI changed from an unscanned port to **Gen1 / Wave**. The installed MB800
implementation confirms that zero detected Gen2 strips causes this explicit
Gen1 fallback. Detection alone does not complete that initialization.

The direct module now checks all four ARGB ports three times; any identified
Gen2 strip or failed detection prevents the conversion. With no Gen2 strips,
it sends command 84, byte6=port, byte7=0 to all four, checks the matching port
acknowledgement, and applies native Off with global output enabled. **The user
confirmed the case fan LEDs went out after this sequence.** Individual case/fan
model identification is no longer needed to operate this verified path.

Both app power and lighting commands perform this bounded initialization. Native
Off uses mode0 and four black slots on the six present areas, leaving controller
output enabled. Brightness0 uses the same operation. Session PowerState retains
the pre-OFF packet, repeated OFF does not replace it, and ON restores it. If no
remembered or currently active effect exists, ON uses static white. The two
asymmetric outbound cycle fields remain 255 and Store remains0. No vendor runtime,
registry profile writes, fan-speed writes or MCU reset is used by the application.

Validation: all **22** Rust unit tests and Clippy passed. The explicit
`msi_regression` hardware run passed power, repeated OFF/ON preservation, exact
readback restoration of static/breathing/spectrum effects, and brightness0 native
Off. Its result is `tools/msi-regression-result.json`; it finished with the fans
OFF. Optical confirmation was supplied for the preceding Gen1 + Off sequence;
the regression's individual effect assertions are controller readbacks.

The board's [official specifications](https://www.msi.com/Motherboard/MAG-B860M-MORTAR-WIFI)
describe three ARGB connectors plus JAF/EZ Conn. Ordinary header-connected RGB
fans need not expose their model or a USB identity to Windows.

## 2026-09-13: wired mouse static logo and app DPI profiles

This supersedes the earlier deliberate exclusion of mouse color effects. In
software mode 2, the native DPI button does not advance its stage. A poll-only
RGB implementation therefore retained a static logo but froze the indicator.
Interface 2, usage FF42:0002 reports `00 02 <buttons>`; rising bit 08 is the DPI
button. The listener ignores other report types and never opens the movement
interface or rewrites button assignments. The user confirmed that the corrected
pink-logo test preserved the logo while DPI indicator colors and pointer
sensitivity advanced with the physical button.

The five native scalar DPI values are read from properties 18..1C, enabled mask
from 1F, current index from 1E, and five **BGR** colors from 2F..33. To select a
current sensitivity, SET 20 carries a little-endian u16 followed by SET 1E with
the index. GET 20 and GET 1E must both match; failed changes restore the actual
previous GET 20 value and index. Native properties 18..1C, 1F and 2F..33 are never
written. No flash, pairing, receiver or keymap command is used.

The exact mouse rejected 1700 (status 3), accepted 1800, and later accepted
200/1200/1800/5200/10000 with exact GET 20 readback. App profiles are therefore
bounded to 200..10000 in increments of 200. Profiles have exactly five values
and RGB colors, a nonempty five-bit mask and an enabled selected index. The app
saves validated settings to its own mouse-dpi.json using a temporary file and
rename, only after successful application. Save errors are reported separately
from successful hardware application. Preferences load at startup without any
automatic hardware change.

OPEN 0D selects handle 0/resource 1. WRITE 06 sends six planar bytes at report
offset 8: indicator R, logo R, indicator G, logo G, indicator B, logo B. Only the
logo is brightness-scaled; OFF makes both slots black while keeping DPI input
handling alive. Heartbeat 12 runs every 10 seconds. A single mutex-protected
runtime owns the mouse; scans, commands and the 8 ms input loop share it. Runtime
errors restore the native current-stage DPI and hardware mode and emit a UI
status event. Normal exit also restores native DPI/mode, retaining requested
illumination ON/OFF. User DPI/color edits are app-only, not onboard saves.

iCUE.exe was observed continuously streaming RGB; it was temporarily stopped
for diagnosis. The shipped runtime detects it through process-name enumeration,
refuses new control, and stops its own active control when a conflict appears.
It never terminates iCUE or alters its services/startup configuration. The wired
runtime regression verified custom profiles, OFF/ON, color replacement, scans
while active, heartbeat, shutdown, and unchanged native presets/mask/colors.
Results: tools/mouse-static-result.json and tools/mouse-runtime-result.json.

Protocol corroboration: [OpenLinkHub HARPOON wired implementation](https://github.com/jurkovic-nikola/OpenLinkHub/blob/main/src/devices/harpoonWU/harpoonWU.go),
[ckb-next Bragi constants](https://github.com/ckb-next/ckb-next/blob/master/src/daemon/bragi_proto.h).
No third-party controller implementation was vendored or linked. UI verification
covered per-device independent drafts, GPU blue versus keyboard yellow, DPI
editing/application and rejection of unsupported 1700 DPI. Twenty-five Rust
unit tests and Clippy with warnings denied passed; the elevated Windows entry
point has no tests and is excluded from test-harness generation.
