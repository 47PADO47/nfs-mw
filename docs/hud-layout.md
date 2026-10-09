# HUD layout

Options > Gameplay > **HUD Layout** changes placement immediately and saves it
when leaving the options screen. It affects the in-game HUD, including the map,
arrow, tachometer, speed digits, nitrous and turbo, and the radio card (artist, title, time), which sits at the
left edge of the HUD: it moves out and scales like the map. Menus keep their own layout.

| Choice | Behavior |
|---|---|
| PC (default) | Uses the stock PC widescreen positions at 16:9, with undistorted gauges. |
| Centered | Keeps the earlier rewrite's centered 4:3 placement. |
| Xbox 360 | Uses the wide positions at 92% scale, matching the Xbox scaling addon. |

All choices use the centered, full-size layout at 4:3 and narrower. Other wide
aspect ratios interpolate or extend the 16:9 placement to keep the HUD visible.
The map's scroll, rotation and world calibration are the same in every preset.

Config file:

```toml
hud_layout = "xbox360" # pc, classic or xbox360
```

Command line: `nfsmw play --hud-layout xbox360`.
Environment: `NFSMW_HUD_LAYOUT=xbox360`.
Console (F12): `set hud-layout xbox360` or `hud-layout xbox360`.
Console changes last for the run; menu changes are saved. These follow the
existing command-line > environment > config-file > default priority.

This reproduces HUD placement and scale. It does not supply Xbox textures,
additional skins, race/pursuit state or the original game's graphics treatment.
The measured native layout and the extension to other aspect ratios are
documented in [specs/hud-viewport.md](specs/hud-viewport.md).
