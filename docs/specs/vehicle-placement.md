# Setting a car on the ground

Sources: static reading of the retail PC executable (private notes), with names from the
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) CC0 reference. No code is copied.

## The rule [binary]

Every reset, jump and race (re)start sets the car down through one placement:

1. The chase camera cuts to the car instead of following it there.
2. The heading becomes the body's forward; world up is the up hint.
3. Linear and angular velocity are cleared.
4. The ground under the car's centre is found.
5. For each wheel, the wheel's horizontal offset in the body frame is turned with the heading and its own ground is found.
   The wheel's corner of the body box goes at `ground + ride height - static sag + half the box height`. The ride height
   is the axle's, plus the ride-height tuning. The static sag is a quarter of the car's weight over the axle's spring
   rate (no sag for a spring with no rate).
6. The orientation is fitted to the four corners. Forward runs from the rear wheels' midpoint to the front ones'; right
   runs from the left wheels' to the right ones'. So the heading stays, and pitch and roll follow the ground.
7. The body's centre goes where the corners put it.
8. The suspension, the engine and the AI are reset. The speed is set to zero, the gear to first, the engine to idle with
   the clutch in, and the wheels stop spinning.

The car is therefore at rest on its springs at once: it neither drops onto them nor bounces. There are no settling
frames and no fade in the placement itself; a jump adds its own fade (`vehicle-reset.md`).

## The rewrite

`blackbox_vehicle::Vehicle::place_on_ground` follows the rule (`vehicle/rest_pose.rs`). Two differences are **[ours]**:
- a wheel with no ground of its own within 2 m of the centre's takes the centre's, where the original fails the
  placement;
- the sag is the linear one the original uses, so a progressive spring settles a few millimetres more.

The traffic's own placement (`place_traffic_moving`) keeps its unloaded clearance. Before this, the player's car was put
down with its springs unloaded and dropped by its sag, which showed as a bounce after every reset and jump.

Tests: `vehicle::rest_pose` (flat, slope, axle sag) and `vehicle::tests::basics::a_car_placed_on_the_ground_does_not_bounce`.
