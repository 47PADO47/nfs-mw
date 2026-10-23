# Rear-view mirror HUD

Status: **drawn, not yet matched to the original.** The mirror is captured offscreen each frame from a rear
camera on the player's car and drawn at the top of the window. The camera placement and field of view are
estimates. Nothing has been compared with the original game's mirror yet.

## Verified in speed.exe

| Address | Name / role | Evidence |
|---|---|---|
| `0x008952f4` | string `RearViewMirrorCameraMover` | camera mover name; xrefs reach only unwind funnels (`0x0086aa42`) |
| `0x00894a60` | string `CM_REAR_VIEW_MIRROR` | camera-mode name; 13th entry of the `CM_*` list, so mode 12 if the list starts at 0 (not checked against code) |
| `0x008ae87c` | string `RVM_MASK` | mask texture name, loaded by `FUN_006bf540` |
| `0x00982b4c` | global, the `RVM_MASK` object | written by `FUN_006bf540`, read by `FUN_006de300` |
| `0x00982b48` | global, the mirror content object | written by `FUN_006bf540`, read by `FUN_006de300` |
| `0x00982b50` | global, cinematic letterbox (**not** the mirror) | `FUN_006d0970` draws two full-width bands gated on the cut-scene amount |
| `0x006de300` | main HUD draw | draws `0x982b48` with blend (5,6), then `0x982b4c` with (2,1), via `FUN_006cff40` |
| `0x006cff40` | HUD quad draw | unit quad transformed by the per-view record at `0x009195e0 + view*0x70` |
| `0x009195e0` | per-view table, 0x70-byte records | read by many functions; `FUN_006c06a0` sets the view-enable byte (`+8`) and the view-data pointer |
| `0x00982a20` | current view index | written by `FUN_006de300` and `FUN_006d33b0` |
| `0x006e7220` | frame view loop | renders each enabled view 0..0x17 through `FUN_006cf400` |
| `0x006cf400` | per-view camera build | near/far from the camera object at `record+0x40` (`+0x2f`, `+0x30`); FOV from `FUN_007c4b80` |
| `0x006c8000` | per-view matrix setup | sets view and projection, and the type-dependent extras |

## Read from the running original (speed.exe, Cheat Engine)

- View table `0x009195e0`, record stride 0x70. Record 3 (`0x00919730`) is the mirror view: type 3, enabled byte 1.
- Its camera object is `0x00983E70` (record `+0x40`).
- Near plane `+0xBC` = 0.75 m; far plane `+0xC0` = 192 m (read in the menu, not in a race).
- The view matrix at `0x00983E70` (rows 0-3) is stale in the menu. Eye, look point and FOV must be read during a race.

## Not yet known

- **Mirror view index.** The spec (`scenery-visibility.md`) says view id 3. Not confirmed in the table: the
  per-view values are written at runtime, so a static read of `0x009195e0` is zero.
- **Mirror camera values.** Near, far, FOV and placement come from the camera object that the mover creates.
  The mover's constructor is in the function that owns the unwind funnels at `0x0086a000–0x0086aa60`; it has
  not been decoded.
- **How the original masks the content.** The `RVM_MASK` pass and the content pass are identified, but the
  on-screen rectangle comes from object data behind `0x982b48`/`0x982b4c`, which is runtime data.

## What the Rust build does

- `viewer/camera/rear_view.rs`: `RearView`, a `Camera` at the car's pose. The eye is `EYE` (0.2, 0, 1.1) in the
  car frame and it looks at `LOOK` (-30, 0, 1). The vertical FOV is 24 degrees (about 70 degrees across the mirror strip; 40 looked like a fish-eye). **All three
  are estimates.**
- `scenes/world/rear_view.rs`: the world as seen from that camera, using the same visibility path as the main
  view. The player's car is left out; traffic and props are kept.
- `Scene::rear_view` (`viewer/mod.rs`): optional; `WorldScene` implements it, other scenes return `None`.
- `app/mirror.rs`: captures the rear view at `384 x 96` through `request_view_capture` (a reduced picture that does not block; the
  default falls back to a full `request_capture`), uploads it as a UI texture,
  and draws it as a quad, 300 points wide, 8 points from the top, over the UI layer. It runs one capture at a
  time: a new one starts when the last one is collected.
- `app/render/mod.rs`: the overlay is updated before the main frame, and the UI layer is re-sent every frame
  with the mirror on top.

## Fade-in

The mirror fades in over 0.6 s from the first frame of the HUD, the length of the game's own mirror track
(`0x5b0d9106`, `len 600`). The game starts that track from an event this build does not raise, so the binding
drives the fade itself: `HudBinding::advance_mirror_fade` (per frame, from the plugin's frame time) and
`mirror_fade_alpha`. The bezel and radar backing are scaled back to full alpha in the presenter (`faded_alpha`).

## Known gaps

- **Cost.** Each frame draws the world twice. The mirror has no setting to turn it off yet.
- **The capture includes the UI.** The mirror shows the on-screen metrics readout, and it would show the
  mirror itself one frame later. The original's mirror has neither.
- **No mask.** The original uses `RVM_MASK` to shape the picture. The Rust build draws a plain rectangle.
  The mask texture comes from the game install and is not part of the repo.
- **Placement is a guess.** The view is not checked against the original, so the framing will differ.

## How to check it

1. `nfsmw view-world --drive --wait-for-load --screenshot out.png` renders one driving frame with the mirror.
2. Capture the same drive in the original game (same car, same spot, same heading) and compare the mirror's
   framing, size and what is visible in it.
3. Once the camera object is read from the mover (see Not yet known), replace `EYE`, `LOOK` and
   `FOV_Y_DEGREES` with those values and record their source here.
