# The 2012 cameras and drifting of the NFS: Most Wanted (2012) content mod

These two sections were part of the [mod's README](../../mods/nfsmw-2012-content/README.md); they moved here to keep it under
the size limit. Everything in them is unchanged.

## The 2012 cameras

Every converted car also gets the 2012 game's cameras as three **extra camera views** ([api-world.md](../modding/api-world.md#camera),
`bb.camera` 1.1): **2012 Chase**, **2012 Bonnet** and **2012 Bumper**. Press **C** (the camera hotkey) past the game's own
cameras to reach them, or pick one in the options' Camera row (on the Player page and on the Gameplay page, which is also the
pause menu's, so the view can be switched in a running race); the choice is saved as
`camera_mode = "extra:nfsmw-2012-content/chase"` (and `bonnet`, `bumper`). On a car the mod did not convert, the views do not fit
and the game keeps its own cameras.

While a car is converted, the mod reads the cameras of its bundle with
[`chameleon-camera`](../formats/chameleon-camera.md) and writes them next to the car's files (`files/CARS/<ID>/CAMERAS.TOML`, with
the nose of the converted body, which anchors the bumper camera; the chase and bonnet offsets are shortened by how far the
conversion moved the car's origin to the middle of its axles, `CarCameras::move_origin_forward`). **A car converted before the
cameras were kept has no such file, and the views then do not fit it (the game keeps its own camera, which looks like the
views having no effect): the conversion version was bumped for this, so the cars are converted again on the first start after
the update and the views appear at the start after that.** The mod logs `no 2012 cameras` for a car without the file. In the game the mod steps that library's model once per frame with the
presented car (the position and axes of the car model, whose origin is the data's: on the ground between the axles, and not the
physics body's centre of gravity; speed, steering, pedals, yaw rate, ground contact, hits) and the host draws from the pose it
answers; the bonnet and bumper views hide the car's own body. The per-car values (the chase offset and height, the look-at point, the
pull-back and field of view with speed, the roll, the braking pitch, the shake) come from each car's own 2012 data; **how the
2012 game turns them into a camera is not known from its executable** (nothing of its camera code was found), so the model is built
from the data and every behaviour is **[unconfirmed]** in the format doc. The mounted cameras' numbers are offsets from an anchor
the data does not name: the mod anchors the bonnet camera at the car's origin and the bumper camera at its nose. None of this has
been seen in a running game yet; the model's tests and the reader's install tests check the numbers, not the look.

## 2012 drifting

In the 2012 game a drift fills the nitrous tank, and the cars slide easily. The mod does both for the converted cars, always, with no
setting (an old saved `option_drift_nitrous` key in the config is ignored). They apply **only while the player drives one of the
mod's own cars**: the mod reads the player's car id from `bb.engine` (`car()`) and compares it with its `cars/<id>.toml` manifests
(`src/drift_cars.rs`, `applies_to`). In a stock car or a car of another mod the mod sends `set_drift_assist(false)` every tick
(also right after a switch of cars) and pays no nitrous; traffic and police cars are never the player's car. A game without
`bb.engine` has no way to tell, so nothing applies. **Drift physics:** while the player's car is a 2012 car the mod keeps its
drift assist on (`bb.vehicles` 1.2 `set_drift_assist`, sent every scene tick): the handbrake with the wheel turned starts a slide, the wheel holds
its angle (full lock about 20 degrees) and the rear tyres let go part of their grip, so the Porsche 918 and the other converted
cars drift; the cars' own numbers (when a slide is a drift, the largest angle, the yaw torque) are `drift_*` keys in their `[stats]`
from their `DriftBehaviour` ([chameleon-drift.md](../formats/chameleon-drift.md#the-sliding-physics)), the 2005 cars have
the generic ones. **Nitrous:** while the player drives a 2012 car, the mod steps
`chameleon_vehicle::DriftModel` once a scene tick with the car (`bb.vehicles` 1.1: speed, slip angle, ground contact,
the strongest hit) and adds what a drift earned to the car's tank with `add_nos`. The rule and its numbers (a slide of 7.5 degrees or more at 10 m/s starts a drift, it
pays 6 units a second of a 100 unit tank from 30 mph after 0.75 s, it ends when the slip stays under 3 degrees for 0.25 s or
the car hits something) are in [chameleon-drift.md](../formats/chameleon-drift.md); the payout is read from the
install's first car bundle when the first 2012 car is driven, the physics are the median car's. While a drift pays, the 2012 text
"DRIFTING" and its distance show on the HUD message line (`bb.hud`). A car with no nitrous system earns nothing, and in a car
that is not a 2012 one nothing is read or changed (`src/drift_nitrous.rs`).

**Drift effects.** The install has no drift camera, post effect or rumble, no free-roam score popup (the score, multiplier and
gate bonus HUD belongs to the Drift Attack event) and no drift-specific smoke or squeal (the tyre smoke, skid marks and noise are
the tyre's response to slip, and are the host's own), so the mod adds none of them; what it has is the "DRIFTING" line above. The
evidence and the open items are in [chameleon-drift-effects.md](../formats/chameleon-drift-effects.md).
