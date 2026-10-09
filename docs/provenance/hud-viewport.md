# HUD viewport presets

- Spec: [hud-viewport.md](../specs/hud-viewport.md).
- Implementation: `crates/nfsmw/src/hud/viewport.rs`; layout setting and menu
  integration in `crates/nfsmw/src/settings/`, `frontend/` and `devtools/console/`.
- Sources: the original-game/rewrite captures supplied by the user; the install's
  `HUD_SingleRace.fng` objects and WIDESCREENMODE responses; the existing
  [HUD minimap spec](../specs/hud-minimap.md) and its credited CC0 decomp/binary
  references; Xbox 360 Stuff v4's installed configuration and a local read-only
  binary inspection; ThirteenAG/WidescreenFixesPack (MIT) for the concept of
  aspect-dependent placement. The full evidence is in the viewport spec.
- Xbox 360 Stuff project page: <https://nfsmods.xyz/mod/1200>, elaymm4 and
  ToruTheRedFox. Reference only; no addon code, decompiler output or assets copied.
- Implemented 2026-10-09 from the written specification; restricted source and
  disassembly were not open while writing the implementation.
- Checks: stock 16:9 gauge/backing transforms compared with the package's actual
  wide response; map/arrow/backing registration through scrolling and rotation;
  numeric reference centers; undistorted intermediate/ultrawide layouts; fresh
  trees across resizing and preset switching; layered config, menu and console
  tests. Native screenshots exercise the real presenter on original and modded
  assets. The user's reference includes an Xbox scaling addon, so its 92% scale
  is an explicit preset rather than a universal PC default.

The preset reproduces the placement of the currently driven HUD elements. It
does not implement the absent race/pursuit features or replace menu rendering.
The host's 16:10 interpolation and ultrawide extension are documented extensions
to the retail game's boolean wide mode.
