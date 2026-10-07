# Scenery level of detail

- **Sources read:** dbalatoni13/nfsmw (CC0, decompiled): `src/Speed/Indep/Src/World/Scenery.cpp` (instance
  culling and model choice), `World/Scenery.hpp` (`InlinedViewGetPixelSize`), `Ecstasy/eView.cpp`
  (`eView::GetPixelSize`), `Ecstasy/Ecstasy.hpp` (`eSolid::NumPolys`, `eSolid::Density`). Read for
  understanding; no code copied.
- **Data inputs:** `SceneryInfo` (4 solid keys, radius) and `SceneryInstance` (position, exclude flags) from
  [../formats/maps.md](../formats/maps.md); `SolidInfo` `NumPolys` (i16 at 0x14) and `Density` (f32 at
  0x9C) from [../formats/models.md](../formats/models.md).

## Behaviour

Each `SceneryInfo` lists up to four models (slots 0–3, most detailed first). The view picks at most one of
them per instance, every frame, from the instance's **projected size in pixels**.

**Projected size.** Use the info's radius plus 6 m as a bounding sphere around the instance position:

```
radius   = info.Radius + 6
to       = position - camera_position
if dot(to, camera_forward) < -radius:  size = 0                  (wholly behind the camera)
away     = |to| - radius
size     = radius * H / away            if away > radius
         = H                            otherwise                  (camera inside or very close)
size     = truncate to integer
```

`H` is the view's projection scale in pixels: the focal length in pixels for the view's vertical field of
view. Where the engine sets `H` (PC platform code) is not decompiled yet. We use
`H = (reference_height / 2) / tan(fov_y / 2)` with a reference height of 480 lines, so detail does not
depend on the window size **[unconfirmed]**.

**Choice for the normal player view:**

1. `size < 2` → not drawn.
2. If the instance flag `0x2000000` is set, add 10 to `size` (biases it towards the detailed model).
3. `size <= 17` → **not drawn**. This is the engine's real draw distance for scenery: small props vanish
   long before large buildings.
4. Otherwise compute a **density score**, 8.7 by default. If slot 0 has a solid with more than 39 polygons,
   the score is `size / max(solid.Density, 6)`.
5. A score below 8.7 draws **slot 2** (the low-detail model), otherwise **slot 0**. If the chosen slot is
   empty, nothing is drawn.

Slots 1 and 3 are only used by other views (reflections, the rear-view mirror, environment maps), which
also use a fixed threshold of 32 px (`0x20`) with slot 2 and, for some flags, slots 0 or 3.

## Constants

| Value | Meaning |
|---|---|
| 6 m | added to the info radius |
| 2 px | below this, skip without further work |
| +10 px | bonus for instances with flag `0x2000000` |
| 17 px | minimum size to draw anything in the player view |
| 39 polygons | below this, the density test is not used (slot 0 always) |
| 6.0 | lower clamp of a solid's `Density` |
| 8.7 | density-score threshold between slot 2 and slot 0 |

## How to check it

- Fly away from a street lamp in the original game and in `nfsmw view-world`, and compare the distances at
  which it switches model and disappears (at 70° FOV the lamp should vanish at roughly
  `(r + 6) · 343 / 17 + (r + 6)` m).
- Large buildings should keep their slot 0 model much farther than dense props.
