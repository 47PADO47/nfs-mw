# Car reset

Sources: static reading of the retail PC executable (private notes), with names from the
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) CC0 reference. No code is copied. The 2 s object-collision
grace is in [reset-object-collision](reset-object-collision.md); the placement in [vehicle-placement](vehicle-placement.md).

## The trail [binary]

The car keeps the last **four** places it drove through, in a ring. Each place holds the position, the body's forward
and the time. A place is recorded on a frame when every wheel is on the ground, the car is over a world surface, and the
car is at least **2 m** from the newest place (or the trail is empty). On a frame that records, no automatic reset is
tested. So while driving, the trail spans about 6 to 8 m of the car's own path; in the air, on two wheels or off the
world it stops, keeping the last good places.

## The reset [binary]

A reset goes to the trail's **oldest** place:
1. It is refused when the trail is empty.
2. A manual reset is also refused while a reset's grace runs, and while the car is held on a grid, driven by the AI or
   seen through a camera other than the drive camera.
3. The place is snapped to the nearest reset lane of the road network. The snap is kept only when the lane is within 6 m
   of the place's height, the move crosses no track barrier, and the ground there is within 3.5 m of the place's height.
   The heading then becomes the lane's (the race direction while racing).
4. Otherwise the place itself and its recorded forward are used.
5. The car is placed there, standing still, and a 2 s grace starts: a manual or automatic one.
6. The place it went to becomes the trail's only entry, so another reset goes there again until four new places are
   recorded.

There is no speed, pursuit or cool-down check, and no fade. The camera cuts to the car. While the grace runs the car
blinks: hidden 0.05 s, shown 0.07 s. Only a manual reset brings the pursuit's reset penalty (the wider bust radius;
[fairbusts-reset](fairbusts-reset.md)). Any other placement (a jump, a race start) ends a grace that runs.

## Automatic resets [binary]

- **Flipped:** not every wheel on the ground, and the car's up axis more than 60 degrees from world up and from the
  ground's normal, for **more than 4 s** without a break. It uses the same path as a manual reset, but a running grace
  does not refuse it.
- **Off the world:** the car's own surface missing and two or more wheels without a surface, for a human driver. It
  resets at once.
- There is no stuck, water or road-distance test. Track zones in the map data can reset the car to a set point.

## Jumps [binary]

A jump to a marker fades the screen to black, puts the car on the marker once the area there has loaded, and fades back
in.

## The rewrite

`scenes/world/drive/reset_trail.rs` holds the trail, the flip timer and the blink; `drive/simulation.rs` runs them each
physics step; `drive_input.rs` serves a reset's placement; `career/jump_fade.rs` fades a jump. Differences, all
**[ours]**:
- the snap takes the nearest road surface (the rewrite's roads have no reset lanes) within 25 m of the place and within
  3.5 m of its height, and does not test barriers;
- the flip test reads world up only;
- the blink runs on simulation time;
- the off-world test is the rewrite's own fall watch (no wheels for 0.75 s with nothing below, or far below the last
  road), which now resets to the trail;
- the jump's fade times (0.3 s out, 0.5 s in).

Before this, a reset put the car on the road nearest where it ended up, any placement granted the grace, a car on its
roof stayed there, and a jump was seen.

Tests: `drive::reset_trail` (the trail, the flip timer and the blink), `drive_input::reset_snap_tests`, and
`drive::placement::tests::installed_player_reset_collision_grace_requires_a_successful_replacement`.
