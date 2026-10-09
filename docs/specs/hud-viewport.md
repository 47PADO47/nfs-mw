# HUD viewport and layout presets

The viewport applies only to the in-game HUD. Front-end menus keep their own layout.
The shared presenter still draws undistorted geometry on a centered, 480-unit-high canvas.

## Sources and evidence

- The user's original-game and rewrite captures at 3840x2160 (2026-10-09).
- **[verified]** `HUD_SingleRace.fng` in both local installs: after its
  `WIDESCREENMODE` response settles, the left context (`0x1603009e`) moves -120
  units and the right context (`0x5d0101f1`) moves +120. Their descendants include
  the map backing and all of the driven gauges respectively. The independent
  `TRACK_MAP` and `PLAYERCARINDICATOR` objects do not move on this message.
- [hud-minimap.md](hud-minimap.md), section 8: the original separately shifts
  the map pieces, its pivot and the player arrow by -120. This is behavior from
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled)
  and locally inspected `speed.exe`; no source code is copied.
- **[verified]** ToruTheRedFox's Xbox 360 Stuff v4, the user's installed
  `scripts/X360Stuff.ini`: `FEScalingMode = 1` specifies 0.92 in widescreen.
  Read-only disassembly of `X360Stuff.asi` corroborates multiplication of both
  FE scale axes by 0.92. SHA256:
  `70171ad75d5ccdf6032bed161fbcdad6a0799b2f7c717af6d977e3571dd185bf`.
  The addon and its code are not shipped or copied.
- [ThirteenAG/WidescreenFixesPack](https://github.com/ThirteenAG/WidescreenFixesPack)
  (MIT), `source/NFSMostWanted.WidescreenFix/Frontend.ixx` and
  `includes/stdafx.cpp`: an aspect-dependent horizontal correction is useful
  beyond the original boolean wide mode. The extension below is the rewrite's
  policy, preserving the stock 16:9 placement rather than that mod's offset.

## Presets

`hud_layout = pc | classic | xbox360`, default `pc`; the same setting is offered
in Gameplay, the console and the environment. `classic` retains the rewrite's
previous centered 4:3 canvas at all window shapes. `pc` places the stock wide HUD
at 16:9. `xbox360` uses that placement and a uniform 0.92 scale in wide windows,
matching the scaling addon used in the supplied original-game reference.
At 4:3 and narrower all presets retain the centered, unscaled layout.

Let `a = width / height`, `wide = 16/9`, `normal = 4/3`. For a wide window:

```
scale = 0.92 for xbox360, otherwise 1
progress = clamp((a - normal) / (wide - normal), 0, 1)
offset = 120 * progress + 240 * max(a - wide, 0) / scale
```

The interpolation to 16:9 and the extension to wider windows are host policies,
not claims about the retail game's fixed 120-unit shift. They prevent clipping
at 16:10 and retain the 16:9 side margins on ultrawide displays.

## Applying it

Classify the left/right contexts, map and arrow, and each one's descendants once.
Translate each resulting **world** transform by its side times `offset`, then
scale x/y uniformly about the canvas origin. Leave depth unchanged. Applying the
translation after the map's rotation is visually equivalent to shifting both
its pieces and its rotation pivot; the mask, roads, arrow and backing stay
aligned in either map mode. Descendants receive the shift exactly once.

Start from a fresh runtime tree every frame. The package's transforms, scripts,
textures, mask UVs and map projection are untouched; repeated frames, resizing
and switching presets cannot accumulate offsets. This implements the measured
wide positions of the currently shown elements without running a second set of
package animation scripts over the live HUD.

At 2048x1152, stock map center (-221,139) becomes (271.072,882.912) with
`xbox360`; the stock tachometer face (221,139) becomes (1776.928,882.912).
These agree with the supplied original-game positions. Different car skins,
mods, blips and world locations remain independent of this viewport.
