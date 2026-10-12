# Camera views

- **Sources read:** `GLOBAL/attributes.bin` of the install (the `camerainfo` class and the `ecar` class, read with
  the `blackbox-attrib` reader); `speed.exe` in Ghidra: the `CDActionDrive` constructor (`FUN_00484c20`), the
  camera setup (`FUN_00481bd0`) and the view selection (`FUN_004713c0`, which switches on the view index and
  looks up each view by name hash); `docs/specs/dynamic-mixer.md` and `docs/specs/car-sound-mixer.md` (the audio
  camera states, which agree with the views below).
- **Data inputs:** the `camerainfo` collections (one per view) and the `ecar` references to them (one per view,
  per car).

## Status

The original game has seven gameplay views, all now in the game as settings: **bumper**, **hood**, **close**,
**far**, **superfar**, **drift** and **pursuit**. The game's default is **far**. They are placed from the install's
own values. The chase and dynamic-FOV cameras are the game's earlier settings and are kept.

Still unconfirmed (see [Unconfirmed](#unconfirmed)): the speed curve between the rest and speed values, the sign
of the offset, whether the field of view is vertical, what two of the fields and one zero field do, and the use of
stiffness. Those are the first things to check in the game.

## The seven views

Index order is the one the camera code switches on (`FUN_004713c0`): the index is set from the car's camera
(the `ecar` field for that view) and selects the view's collection.

| Index | View (camera setting) | `camerainfo` collection | `ecar` field (name hash) | Collection key |
|---|---|---|---|---|
| 0 | bumper | `bumper` | `0x585517F3` | `0x1F4B1746` |
| 1 | hood | `hood` | `0xD74C1435` | `0x201893D0` |
| 2 | close | `close` | `0x0C2DA793` | `0xC6ECEB8C` |
| 3 | far (the default) | `far` | `0xCCF03CB3` | `0x18EC50C6` |
| 4 | superfar | `superfar` | `0x10204A90` | `0x32C8A5FF` |
| 5 | drift | `drift` | `0x4B675DC8` | `0x29076EF4` |
| 6 | pursuit | `pursuit` | `0xD76A6FAD` | `0xDAA252C2` |

The default case in `FUN_004713c0` sets the index to 3 (`far`) when the requested view is not one of the seven.
The mapping was checked on all 100 `ecar` collections: their 700 camera references each resolve to the view in
the table, either to the generic collection or to a car-specific variant of it (for example `bmwm3gtr_hood` or
`van_close`). The code uses the car's own variant of each view when the install has one, and the generic view otherwise.

## Fields **[verified names, inferred meanings]**

Each view collection holds nine attributes. Five are the camera's values and are used by the game (the field names
below were resolved by matching the field's `vlt_hash` against these names; see [Names](#names)). Each of the five
is a pair: the value at rest and the value at speed.

| Field | Pair values by view: bumper / hood / close / far / superfar / drift / pursuit | Used as |
|---|---|---|
| `FOV` | 83, 90 / 80, 90 / 78, 78 / 90, 90 / 78, 78 / 29.71, 86.04 / 65, 86.04 | field of view, degrees |
| `HEIGHT` | 0.746, 0.61 / 1.397, 1.2 / 1.62, 1.7 / 1.7, 1.85 / 5.0, 1.85 / 2.04, 3.23 / 1.5, 3.23 | height above the car, metres |
| `LAG` | 0.5, 0.716 / 0.24, 0.25 / -4.7, -6.8 / -5.44, -8.2 / -20, -8.2 / -15.49, -8.2 / -9, -8.2 | offset along the car, metres |
| `ANGLE` | 1.5, 0.03 / 2.8, 3.08 / 0, 0 / 0, 0 / 0, 0 / 0, 11.13 / 0, 11.13 | pitch, degrees |
| `STIFFNESS` | 0.5, 0.5 / 0.98, 0.96 / 0.5, 0.5 / 0.5, 0.5 / 0.5, 0.5 / 0.5, 0.5 / 0.5, 0.5 | the stiffness of the view, 0..1 |

The default view `default` (the fallback in the game's code) has the values FOV 78/78, HEIGHT 2.0/1.0, LAG -10/-10
and ANGLE 0/0. It is not one of the seven views and is not selected by the camera setting.

Other attributes of each view, with values the same for every view or unconfirmed:

| Field | Values | Status |
|---|---|---|
| `0x2FC5F041` | 0, 0 in every view | unnamed; zero everywhere, so unused in the views seen |
| `0x665F4D74` (bool) | true, true in every view | unnamed |
| `0x40E94F86` (bool) | false in `default`, `drift`, `pursuit`, `superfar`; true in `far`, `close`, `bumper`, `hood` | unnamed; differs by view |
| `0x9CA1C8F9` (text) | the view's name | the collection name |

## Behaviour

### Choosing a view

The camera setting cycles through, in order: chase, dynamic-fov, bumper, hood, close, far, superfar, drift,
pursuit, then wraps. The options row shows Chase, Dynamic FOV, Bumper, Hood, Close, Far, Super far, Drift and
Pursuit. File and environment names are `bumper`, `hood`, `close`, `far`, `superfar`, `drift` and `pursuit`.

The camera hotkey changes the setting only while driving (the existing rule). An original view applies only in
the chase view of a car; with no car the free camera is used. If the install's views cannot be loaded, the
setting falls back to the chase camera.

### Placing a view

The views are loaded for the car that starts driving: its own `<car>_<view>` collection where one exists
(the car's name in lower case), otherwise the generic view. A car not in the install uses the generic views.

For a car at its interpolated pose (model origin `position`, heading `heading` from +X, speed `v` in m/s):

1. The speed fraction is `t = clamp(|v| / 60, 0, 1)`. Sixty metres a second is the chase camera's full-speed value;
   the game's own curve is unconfirmed.
2. Each value is `rest + (speed - rest) * t` for the pairs above.
3. The eye is `position + (cos heading, sin heading, 0) * (LAG + start) + (0, 0, HEIGHT)`, where `start` is 0 for a
   negative `LAG` and the car's half length (the physics box's `half_extents().1`, about 2.2 m) for a positive `LAG`.
   The positive case is inferred from screenshots: with `start` at 0 the bumper and hood views sat inside the car's
   body (the dashboard and windscreen filled the picture). Starting them at the front bumper put both outside the
   body, with the bumper low and the hood higher. The far, close and superfar views (negative `LAG`) show the car
   from behind, as expected. The bumper's `LAG` of 0.5 m and the hood's 0.24 m are measured from the front bumper,
   which the game's own data does not state.
4. The view points along `(cos heading, sin heading, -sin ANGLE)`, so a positive `ANGLE` looks down.
5. The lens uses the FOV, the near plane of 0.3 m and the window's aspect ratio.

There is no smoothing, no obstacle handling and no use of `STIFFNESS` yet.

## Names

The names were resolved by matching the `vlt_hash` (Jenkins' `lookup2`, initval `0xABCDEF00`, the crate's
`vlt_hash`) of candidate names against the class's field keys. Only uppercase forms matched: `FOV`, `HEIGHT`,
`LAG`, `STIFFNESS` and `ANGLE`. The per-car `ecar` fields for the seven views were matched by resolving their
`RefSpec` values to the collection names above. The view order was taken from the getter functions in the game's
code (`FUN_00467d70` to `FUN_00467e30`), which pass each view's name hash to the lookup.

## Constants

| Constant | Value | Source |
|---|---|---|
| Speed fraction full scale | 60 m/s | the chase camera's `FAST` (not confirmed for these views) |
| Near plane | 0.3 m | the chase camera's `NEAR_PLANE` |

## Unconfirmed

- **Speed curve.** The values are interpolated linearly with a 60 m/s scale. The game may use another curve or scale.
- **Where positive `LAG` starts.** Positive values are taken to start at the front bumper (see step 3). This is
  inferred from screenshots of the bumper and hood views, not from the game's code. Check it against the original.
- **Field of view.** Taken as vertical degrees. The game may use horizontal degrees.
- **`ANGLE` sign.** Taken as a pitch down for positive values. The values are small, so this is worth checking.
- **`STIFFNESS`.** Not used. Its use in the game (probably how firmly the view follows the car) is not known.
- **The unnamed fields.** The zero field and the two booleans are not used.

## Known gaps

- **The car body may hide the view.** The car model is drawn opaque, so the bumper, hood and close views may show the
  car's own body from inside. Not handled.
- **No obstacle handling.** A view can clip through a wall or the ground, unlike the chase camera.
- **Not checked in the game.** The values and the mapping are from the install's data and the game's code. Nothing
  here has been seen in the running game yet.

## How to check it

1. Cycle the camera setting in the game with a car and compare each view with the original game. The bumper, hood and
   far views are the easiest to tell apart.
2. For the sign of `LAG` and the field of view, compare the framing of the far view (default) at rest and at speed.
3. For the speed curve, drive at a few speeds and compare the camera height and distance with the pairs above.
4. Record each result here, with the setting used and the car.
