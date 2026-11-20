# CustomHUD gauges

- **Spec:** [custom hud gauges](../specs/custom-hud-gauges.md); the file format is
  [custom-hud-skin](../formats/custom-hud-skin.md).
- **Sources:** Ghidra on the `CustomHud.asi` of the inspected install (2022 build, 328 KB, project `nfsmw`, an x86
  Visual C++ 2019 build with the MirrorHook Direct3D 9 extension); the skin folders and ini files beside it; one screenshot
  of the plugin running in the game. The plugin's licence is unknown, so nothing is copied: the spec states behaviour in our
  own words, the tests use skins written for them, and no picture, ini or binary is in the repository.
- **Addresses read in `CustomHud.asi`** (image offsets, base `0x10000000`):
  - Start-up and games: `FUN_10003970` (entry point selects the game binding), the Most Wanted vtable `0x10047AB4`
    with its getters `0x10014DE0` to `0x100151E0` and the car-name getter `0x10015250`; the hook wrapper `0x10014C60`
    with `0x10014BB0` (the flag mask).
  - Settings and skin choice: `FUN_10008130` (reads `scripts\CustomHud.ini`, chooses the general or per-car skin),
    `FUN_10007FB0` (the per-car path), `FUN_10007B70` (file exists), the ini reader `0x10005160`, `0x10005640` (integer),
    `0x10005900` (float; its default comes in `XMM3`), `0x10005B00` and `0x100064A0` (strings).
  - Sections and keys: `FUN_10006F60` (sections), `0x10006A50` (gauges), `0x10006690` (digits), `0x10006960` (the speed
    and rev digits), `0x100067A0` (bars). The float defaults were read at each call (`ArrowScale` 1.0, `ArrowCenterOffset`
    0.22, `NumbersMaxThreshold` -1.0, the two mask sizes 128, `Scale` 1.0, everything else 0).
  - Drawing: the sprite transform `0x1000E7A0` (the 1080-line reference, the right and bottom anchor, the rotation),
    the sprite draw and its blend states `0x1000E330`, the texture loader and cache `0x1000E230`, `0x1000E5E0`, the mask
    sprite `0x1000E9F0` and its sector fill `0x1000EAC0`; the gauge `0x1000EEB0` (build), `0x1000F110` (layers),
    `0x1000F620` (filled arc), `0x1000F7E0` (needle); the digit cell `0x1000FCA0`, the speed digits `0x10010000`, the
    gear callback `0x10004020`, the digit value callbacks `0x10004060` to `0x10004220`; bars `0x100106E0`, the shift light
    `0x10010930`; the HUD builder `0x10010AE0` (which getter feeds which gauge) and the frame `0x100113A0` (draw order);
    the frame hook `0x10012240` (visibility, car change, hot reload, debug text `0x10011C30`).
- **Implementation:** 2026-11-07, from the spec: `libs/blackbox-gauge-skin`, `hud/asi_skin/`, `settings/custom_hud_skin.rs`.
- **Checked:** unit tests with ini text written for them (parser, angles, digits, masks, placement, detection against a
  temp folder). An ignored test (`hud/asi_skin/install_tests.rs`) parses and decodes every installed skin. Not run against
  the running original and not compared pixel by pixel with the plugin.
