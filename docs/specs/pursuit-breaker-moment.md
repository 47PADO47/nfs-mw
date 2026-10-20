# The cinematic moment of a pursuit breaker

What the original plays when a breaker structure collapses in a chase, and what the rewrite does about it. Method and
addresses: [provenance](../provenance/pursuit-breaker-moment.md). Evidence tags as in
[docs/README.md](../README.md#evidence-tags): **[verified]** read from the install's data, **[decomp]** read from
`speed.exe` or the CC0 source reconstruction, **[inferred]**, **[open]**. Nothing here was compared with a running original.

The breaker sequences ([pursuit breakers](pursuit-breakers.md)) issue an `EPlayRaceNIS` event of scene type "Moment" next
to the cop zone. It names a scene (`WMGasStation`, `WMBoat`, `WMWaterTower`...).

## 1. A moment is a camera cut, not an animation

The NIS activity treats the type "Moment" apart from every other scene **[decomp]**. It does not load the scene's
animation bank, its skeletons or its audio stream. It creates a small scene object that owns a scene hash, a camera
track number, two matrices and a clock, and goes straight to playing. The scene object is an ICE camera scene: the
only thing it controls is the camera. The structure falls by the world's own physics and event sequences, which keep
running. The `Scene_WM*_BundleB.bun` files in `NIS/` are therefore not needed for the cut.

So the NIS animation codec being open ([formats/animation.md](../formats/animation.md)) does not block the moment.
What the cut needs is the ICE camera data, which is decoded (section 4).

## 2. When it plays

The event runs a gate before anything else **[decomp]**; the cut does not play when any of these fails.

1. No other NIS is playing (the activity is a singleton).
2. The player's car touched the structure within the last 2 seconds. The clock is set when the model's trigger event
   is consumed and the model was last touched by the player's vehicle. The rewrite uses the last contact between the
   player's car and any prop (`drive/touch.rs`) **[inferred]**: it has no per-model "last touched by" yet.
3. An options flag (a byte of the profile's gameplay options, 1 by default) is on. Its menu name was not found; the
   rewrite treats it as on **[open]**.
4. Not the online or split-screen mode, no editor, and in a race the race's own flag allows moments. Free roam passes.
5. The player's car is not being arrested **[inferred]**: the player object's busted flag stops the event.
6. A ground police car that is active and in pursuit of the player is close. The reach is
   `20 + 30 * (1 + cos)` metres, with `cos` the dot product of the heading of the player's car and the direction
   from the cop to the player. A cop right behind has 80 m, one right ahead 20 m, one beside 50 m. The distance is
   horizontal **[decomp]**; which vector is "the heading" is read as the car's forward axis **[inferred]**.
7. A marker of the scene's kind scores under 75 (section 3).
8. The scene has a camera track in the install (section 4).

The test for 2 and 6 means a cut only plays for the player's own collapse with a chaser close behind, never for a
structure a cop knocks over.

## 3. Where it plays: the marker

Each moment scene has a marker kind in the scene candidate table of `GLOBAL/InGameB.bun` (`SceneCandidateData`, count
at 0x14, `(scene hash, kind)` pairs from 0xC9C) **[verified]**. The 21 kinds are named: `GasStationMoment`,
`WaterTowerMoment`, `PorchCollapseMoment`... The track places `TrackPositionMarkers` of those names (`L2RA.BUN`,
0x30-byte records after one alignment word: name hash at 8, position at 0x10, 16-bit angle at 0x20) **[verified]**:
9 gas stations, 3 water towers, 2 porches, 2 boats, 1 each of drive-in, trailer park, amphitheatre, strip mall,
and so on. The 14 installed moment scenes all have a kind and a camera track.

The marker is the one with the lowest score: its distance to the player's car plus `25 * t`, with `t` the turn
between the marker's angle and the car's heading as a share of a turn (0 to 0.5; the angle is counted from forward
towards left) **[decomp]**. The score must be under 75 **[decomp]**. A pump hit at the far end of the city plays
nothing because no marker is close.

The scene is stood on that marker **[decomp]**: its origin is the marker's position with the height replaced by the
ground height under it, and it is turned about the up axis by the marker's angle less a quarter turn. A scene point
`(x, y, z)` is `origin + Rz(turn) * (x, y, z)`. The scene's `+y` axis is the way the camera looks, so a marker's
angle is the direction of travel the cut was authored for.

## 4. The camera track

A camera pack set (`0x8003B200` chunk, key = hash of the scene name) holds the scene's tracks `Track 0`, `Track 1`...
The key layout and the evaluator are specified in [formats/animation.md](../formats/animation.md) section "ICE camera
keys"; the rules the cut uses:

- A track has a length in seconds (3.0 to 3.33 for the moments) and keys. All 14 moment tracks have one key. A key
  holds the eye, look point, dutch, lens and others at the start and at the end of its interval **[verified]**.
- `t` is the elapsed time over the length; outside 0 to 1 there is no camera. The key is the last whose time is not
  after `t`; `u` runs 0 to 1 over its interval **[decomp]**.
- Eye, look, dutch and lens are cubic Hermite curves from the start value to the end value when the key's cubic flag
  is set, straight lines otherwise. A key's tangent at each end is its chord times the key's tangent length (both 1 in
  the moments, which makes the curve a straight line); a key of kind 0 has none **[decomp]**. Smoothing a curve
  across neighbouring keys is not applied (no moment track has two keys).
- The field of view is `2 * atan(15.96 / lens)`, lens in millimetres: 24 mm is 67.2 degrees, the porch's 74.5 mm is
  24.2 degrees **[decomp]**. The rewrite feeds it to the same native optics as the follow cameras.
- Dutch is cut to a whole 16-bit angle unit before use, so the authored 0.002 and -0.003 are zero **[decomp]**.
- Per track, the simulation speed is a percentage: 100 for most moments, **60 for the water tower** **[verified]**.
  The engine writes the camera's simulation time multiplier from it, clamped to 0.01 to 1 **[decomp]**.
  The consumer of that field was not located; the rewrite slows the world by it **[inferred]**, which makes the
  water tower's cut run 5 seconds of real time.
- Letterbox (a percent, times 0.00125 for the bar height) and aperture are read; all moments have letterbox 0 and
  aperture 37 (depth of field off, the engine's test is `37 <= value` off) **[verified]**.
- The scene's camera tracks are chosen by the car's facing only when a scene has two or more tracks **[decomp]**;
  none of the moments does, so track 0 plays.

## 5. What the world does during the cut

| Effect | Evidence | Rewrite |
|---|---|---|
| The camera is the track's (section 4) for the length of the track, then the gameplay camera returns | [decomp] | Yes (`moment.rs`) |
| The HUD is hidden | the HUD group visibility routine hides every group while a NIS exists [decomp] | Yes |
| The busted meter does not move | [decomp] (CC0 `AIPursuit`: the increment is zeroed while a world moment plays) | Yes (`hold_busted`) |
| The world runs at the track's speed | [verified] data, [inferred] consumer | Yes |
| The player's car is driven by the racer AI for the cut: the activity posts `EBecomeAiCar` on loading a Moment and `EBecomePlayerCar` on release | event construction [decomp]; its handler was not located, `SetAiControl(true)` assumed [inferred] | **No**: the rewrite has no player autopilot; the player keeps control |
| No new traffic is created while a NIS exists (traffic density is 0) | CC0 `AITrafficManager::ComputeDensity` [decomp] | No |
| Cop spawning is off but removal is allowed during a moment | CC0 `AICopManager::VehicleSpawningEnabled` [decomp] | No (the cut lasts 3 s) |
| Camera noise (shake), depth of field, letterbox | the key fields exist; all moments have them zero or off [verified] | Not applied |
| Sound of the scene | none: the stream is queued by the load path that a Moment skips [decomp] | None |

## 6. Not verified

- No live comparison with the original; the camera positions are the data read through the rules above. The installed
  test places the gas station's cut on its nearest marker: the look point is 0.4 to 1.7 m from the station's roof for
  the whole cut and the eye stands 12 to 18 m away, which confirms the rotation sense and the quarter-turn offset **[verified]**.
- The touch rule (section 2.2) is a rewrite approximation of the trigger event's "last touched by".
- The options byte (2.3) and the player autopilot (section 5) are open.
- Scenes with no installed camera track (`WMGazebo`, `WMFishMarket`...) play nothing, as the original's gate.
