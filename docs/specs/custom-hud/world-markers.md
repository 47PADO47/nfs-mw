# Custom HUD: world-space markers

Markers that belong to a place in the world (checkpoints, event entrances, pursuit breakers, sighted threats) are
projected through the gameplay camera and drawn in the `WorldMarkers` slot (18), with edge arrows for targets outside
the view. Screen-space information stays in the HUD zones. Built in PR 9
(`crates/nfsmw/src/ui/theme/custom/markers/{project,rules,draw}.rs`). Off with `[custom_hud] world_markers = false`.
Index: [README.md](README.md).

This is a different thing from the ground circles at shops, safe houses and race starts
([world-markers.md](../world-markers.md)), which are 3D props of the world and stay as they are.

## 1. Projection

- Through the gameplay `Lens`'s `view_proj` and the viewport (`camera_choice.rs`), copied into `HudState` in PR 3a
  **before** camera shake is applied. Markers therefore sit on their world positions without swimming with the shake,
  and screen text does not jitter.
- The `hud::sync` system runs after `render::update_scene` (ordering fixed in PR 3a), so the matrix and the state are
  from the same frame.
- A point behind the camera (clip `w <= 0`) is not drawn on screen; if it is a target that needs guidance it gets an
  edge arrow (section 3).
- FOV changes (`DynamicFov`) are in `view_proj`, so markers follow them exactly.

## 2. Marker kinds and rules

| Kind | When | Look |
|---|---|---|
| active checkpoint | racing; the next gate | `nav` gate mark with the distance; emphasised (larger, full alpha) within 150 m and near a junction, with the direction to take |
| next checkpoints | racing; the two gates after the active one | small, `nav` at 0.35 alpha, no label |
| checkpoint passed | on passing | the gate mark expands and fades over `pulse` in `success` |
| missed-checkpoint recovery | the missed checkpoint event until recovered | an arrow toward the missed gate, on screen or at the edge |
| event entrance | free roam; an undiscovered or available event start within 300 m | icon and name; the interaction prompt inside the entry zone |
| discovered location | free roam, Advanced; within 300 m | icon only |
| pursuit breaker | a chase is running and a breaker is within sight distance | small `caution` icon at its base |
| sighted threat | a sighted cop or the helicopter outside the view | edge arrow only ([fair-knowledge.md](fair-knowledge.md)) |

No permanent racing line is drawn on the road.

## 3. Edge arrows

A target off screen, or behind the camera, is shown as an arrow on an inset ellipse inside the window (the window,
not the content box, since the arrow points at the world). The arrow points along the direction from the screen
centre to the target's projected direction (behind the camera the direction is mirrored). Its label is the distance.
Confirmed threats draw solid, last-known ones hollow and fading.

## 4. Styling

- **Size** is in HUD units, so a marker is the same apparent size at every resolution. It scales with distance between
  1.0 (near) and 0.6 (far) and never below.
- **Distance styling:** full alpha within 50 m; labels hidden beyond 300 m; marks fade to 0.35 alpha at their range limit.
- **Text** positions are snapped to whole pixels and low-pass filtered (one pole, about 50 ms) on screen, so camera
  motion does not make labels shimmer.
- Marks respect `[custom_hud] opacity` and the palette tokens; high contrast adds the plate outline to labels.

## 5. Priority and cap

At most **6** markers and **3** police edge arrows at once. When more qualify, the order is: active checkpoint,
missed-checkpoint recovery, pursuit breakers in sight, event entrances, next checkpoints, discovered locations; within a
kind, nearest first. **[ours]**

## 6. Occlusion

- Hidden behind the camera (section 1).
- **Through geometry:** a `blackbox-collision` ray from the camera to the marker against the static world, if the scene
  exposes that query to the HUD; a marker whose ray is blocked draws at 0.3 alpha without its label (checkpoints keep
  guidance through walls; event entrances hide). If the ray is not exposed, occlusion is by distance fading only, and
  this section records it. **[unconfirmed]** until PR 9.
- No outline-through-walls feature is configured.

## 7. Tests (PR 9)

- Projection of known points with a known matrix; behind-camera rejection.
- Edge clamping on the inset ellipse at 16:9 and 32:9; the mirrored direction behind the camera.
- The cap and its priority order; nearest first within a kind.
- Distance styling bands; pixel snapping.
