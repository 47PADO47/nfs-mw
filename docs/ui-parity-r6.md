# UI repair batch R6: platform turn and room atmosphere

Repair candidates from the R4 playtest follow-up. User acceptance remains open.

## Changes

1. **The platform's room turn comes from the executable.** The original draws a platform's solids through one matrix:
   a turn about the vertical axis by a per-platform angle, then a shift along +y. The car stand and the camera are not
   turned. The crib and the shop backroom turn 134.41 degrees, the car lot shifts 0.075 m, and the shop and the car lot
   do not turn. The earlier fitted 180-degree crib turn is withdrawn. The scenery light now stays in the world frame,
   as the original's lighting object does.
2. **Room atmosphere.** Each platform file holds its own particle placements (an emitter library in the front end's
   section). The menu now runs them through the world's ambient effect system:
   - the crib's floor steam and distant birds;
   - the shop's smokestack and steam;
   - the backroom's steam;
   - the car lot's leaves.

   While particles run, the menu draws every frame instead of keeping a still picture.

## Evidence

- The settled crib main menu now matches the modded and the unmodified recordings: the car's heading, the silos on
  the right, the lamps and the forklift. The crib's steam rises behind the car on the right, under the silos, where both
  recordings show it.
- Steam density follows the installed PC groups. The modded reference's thicker smoke is mod content and is not
  reproduced.

## Open

- **Career safe house orientation.** Resolved in R7 (`ui-parity-r7.md`): the remake mirrored the room through the
  solids' pivots.
- **Backroom.** Its new turn is from the executable but has not been compared with a recording.
- **Unnamed groups.** One shared group (crib, shop and car lot) and one backroom group have no recovered names. They run
  by key, as the original resolves them.

## Checks

- A new installed regression checks that the crib's groups reach the renderer and release their textures. It passes on
  the modded and the unmodified installs.
- Stage, room, light, backdrop, garage scene, look and ambient-effect tests pass on both installs. The only failure is
  the known modded-only texture-size assertion.
- Formatting, guards and native Clippy pass, with no Clippy sites in changed files.
