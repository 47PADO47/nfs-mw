# Chameleon drift effects: what the 2012 game shows and plays while a car drifts

[chameleon-drift.md](chameleon-drift.md) holds the rule (when a slide is a drift, what it pays). This page holds the other half
of the question: **what else changes on screen and in the sound while a car drifts** in NFS: Most Wanted (2012), and which of it the
rewrite can draw. The short answer is that the install has little: one HUD line in free roam, a full score HUD in the Drift Attack
event only, and per-car tyre smoke, skid mark and tyre sound numbers that do not name a drift. There is no drift camera, no drift
post effect and no drift rumble in the data. Evidence tags: [README.md](../README.md#evidence-tags), plus **[exe]** for what
was read in `NFS13.exe` (Ghidra, project `nfsmw`, addresses are virtual addresses) and **[scan]** for a negative result of the
scan described below. A number that is not in the data is not given.

## Short answer

| Question | Answer | Tag |
|---|---|---|
| Free-roam HUD while drifting | the one line `DRIFTING {Distance(DIST)}` (string 24010), with the nitrous bar; no score, multiplier or bonus | [verified] text; "no more" [scan] |
| Score, bonus, multiplier popups | only in the **Drift Attack** game mode (`DriftHud` screen, `HSMDoDriftScoring`), not in free roam | [verified] strings and bindings, [exe] code |
| Camera | no drift field in any camera object; the chase camera follows the car's heading with a yaw-rate lead, which already shows a slide | [scan], [verified] camera data |
| Post effects (blur, speed lines) | no drift reference in `POSTFX.BNDL`, `GLOBALEFFECTS.BNDL` or the shaders | [scan] |
| Tyre smoke | no drift-specific emitter; the smoke is the slip regimes of the wheel bindings. "Drift Tires Pro" is described as "emits colored tire smoke" | [verified] text, [data] emitters |
| Skid marks and tyre noise | per-car `TyreVfxBehaviour` and shared `TyreSoundParams`, driven by the lateral and longitudinal slip, with no drift flag | [verified] numbers, roles [unconfirmed] |
| Controller haptics | nothing in the data | [scan] |
| The vehicle's "is drifting" flag | `IsDrifting` ("Is the players race car in a drift?") is a property of the player state; nothing in the bundles reads it | [exe] |

## The scan

Every `.BNDL` of the 2012 install (v1.4.0.0: cars, tracks, `UI`, `SOUND`, `HAWAII`, `GAMELOGIC`, `POSTFX`,
`GLOBALEFFECTS`, `CAMERAS`, `PHYSICS`, ...) was opened with `chameleon-bundle`, every block of every resource inflated, and searched
for the bytes of `drift` (ASCII and UTF-16, any case) and of `skid`, `squeal`, `slip`, `tyre` and `tire`. The resources with a
match:

| Where | What matched |
|---|---|
| each car bundle (93) | the types `DriftBehaviour` (and its six structures) and `NitrousParameters.Drifting`, nothing named drift in the objects ([chameleon-drift.md](chameleon-drift.md)) |
| `GAMEPLAY.BNDL`, `VEHICLELIST.BNDL` | the types `NitrousParameters.Drifting` and `Gameplay.DriftMarker` |
| `GAMELOGIC/GAMEMODES.BNDL` | the Drift Attack game mode machine ("Drift Game Mode", "Drift HUD", "Drift Update", "Drift Game Flow", `HSMDoDriftScoring`) |
| `UI/SCREENS2` (a dozen bundles) | the `DriftHud` screen, the `DriftScoring.*` bindings, the car stat rows `DriftStat`, `DriftModdedStat`, `DriftImprovement` and `RolloutDrift` |
| `UI/LANGUAGE` | the text tables (below) |
| `NFS13.exe` | the names of the stats and view model properties (below) |
| **no match** | `CAMERAS.BNDL`, `POSTFX.BNDL`, `GLOBALEFFECTS.BNDL`, `GLOBALCONFIG.BNDL`, `SOUND/*.BNDL`, `HAWAII/SURFACESOUNDS.BNDL`, `HAWAII/SURFACELIST.BNDL`, `METALCAR.BNDL`, the shader bundles, any car sound graph |

The sound graphs and the surface sounds name no drift. The skid and tyre names that matched are the tyre numbers described
under [Tyres](#tyres-smoke-skid-marks-and-noise).

## The HUD

### Free roam: the one line **[verified]**

String 24010 of the English table is `DRIFTING {Distance(DIST)}`. The nitrous view model of the executable
(`NitrousViewModel`, registered at `0x658EE0`) has `DistanceDrifting` (a float at +0xF4), `DistanceDonutting`, `DistanceOncoming`,
`DistanceTakingShortcut`, `NearMissCount` and `OncomingNearMissCount` next to `NitrousAmount`, `UsingNitrous` and
`NitrousReadyPulse` **[exe]**: the HUD shows the distance of the running drift (the player's distance unit; the English strings
write yards, "a 250 yard (228.6 meter) drift") for as long as the drift pays. The mod's HUD message (`bb.hud`) does the same with
metres. Nothing else of the free-roam HUD names a drift: no score, no combo, no bonus. The other strings of the tables that
mention a drift are milestones and speedlist entries ("Drift 150 yards", "LONGEST DRIFT ON PIER", "NEAR MISS WHILST DRIFTING",
"DRIFT KING"), the mods' descriptions and one tip (below).

### Drift Attack: the score HUD **[verified]** strings and bindings, rule **[exe]**

The Terminal Velocity pack adds the *Drift Attack* event ("Gain as much SP as you can by drifting for as long as possible, and
score bigger amounts of SP by successfully drifting through the markers", string 2244057). It has a game mode machine in
`GAMEMODES.BNDL` and its own HUD screen, `DriftHud` (object `0x212861` of `UI/SCREENS2/2173025.BNDL`, with the screens that include
it; a layout of the usual `HudSelect`, `Speedwall`, `Prompt`, `MapOverlay` group). Its texts, from the English table:

| Id | Text | Meaning |
|---|---|---|
| 2211017 | `DRIFTING` | the label while a drift scores |
| 2211018 | `x{DriftScoring.Multiplier}` | the multiplier (the gates passed in a row) |
| 2216441 | `+{DriftScoring.Drift} <icon>` | the score of the running drift |
| 2211064 | `GATE BONUS +{DriftScoring.Bonus}` | a gate (a `DriftMarker`) passed cleanly |
| 2256112, 2256125 | `+{DriftScoring.Bonus}` | the same, small |
| 2256106, 2256127 | `X {DriftScoring.GatesSuccessfullyPassed}` | the gate count |
| 2256059 | `DRIFT TOTAL +{DriftScoring.LastCombo}` | the combo paid out when the drift ends |
| 2256092 | `DRIFT FAILED` | the combo lost (a crash into a gate, or a gate missed) |
| 2216445, 2216449 | `{DriftScoring.BankedScore}`, `{DriftScoring.TargetScore}` | the banked score and the medal target |
| 2256132 | `DRIFTING:` | the label of the results line |

The screen's bindings (`DriftScoring.Drift(GTEQ:1)`, `.JustLostCombo`, `.DriftJustTickedUp`, `.HasJustHitGate`,
`.WasLastGateASuccess`, `.GatesSuccessfullyPassed`, `.DriftMarkerUsed[#]`) switch its widgets. The class behind them,
`HSMBehaviourHSMDoDriftScoring` (properties registered at `0x756D60`, object size 0x1340), has: `Drift` (+0x1310, int),
`BankedScore` (+0x1314), `LastCombo` (+0x1318), `CurrentGateScore` (+0x132C), the gate count (+0x1328), `Bonus` (the
multiplier, a getter), `TargetScore`, `MedalIndex`, `TargetMedalIndex`, `ScoringActive` and the flags at +0x1331 to +0x1335
**[exe]**. The game mode is an *event*: the HSM starts it with the event, the mod API has no game modes, so the screen cannot
appear in free roam.

**What the code does with the score, as far as it was read [exe], [unconfirmed] units.** A tick function (`0x74CBF0`) adds to the
running score `clamp(a, lo, hi) / hi * k * speed` (times a small count it also computes), where `a` is a float that the
vehicle's `+0x48` method returns (read as the slip angle in degrees), `speed` the length of the vehicle's velocity vector, and
`lo`, `hi`, `k` fields of the `GameLogicConfig` record: **7.5, 45 and 0.725** (the record is object `0x45E6F` of
`GLOBALCONFIG.BNDL`, fields at +0x22C (45), +0x230 (50), +0x234 (0.5), +0x238 (7.5), +0x23C (0.725)). So the score grows with
the speed and with the slip angle from the 7.5 degrees that also start a drift, up to 45 degrees. At the end of the drift
(`0x756C20`) the running score is rounded down to a multiple of a small integer step of another config record, added to the gate
score and to the banked score, and kept as `LastCombo`. How often the tick runs, what the count is, the step, and what ends a
combo are not known, so **the score is not ported**.

### Tips and tutorials

String 1909535 `TAP BRAKE TO DRIFT` and 1682566 "Cops won't ignore dangerous driving; hit traffic or pull a drift near a patrol car
to start a pursuit" are loading tips or hints. The trigger of the first is not known (the Drift Attack start, most likely). Both
are text only.

## Tyres: smoke, skid marks and noise

None of this names a drift: it is the tyre's own response to slip, and a drift is a slide, so it smokes, marks and squeals.

### Smoke and skid marks: `TyreVfxBehaviour` **[verified]** layout and numbers, roles **[unconfirmed]**

Each car bundle has one `Genesys.Gen.TyreVfxBehaviour` object (928 bytes) of two copies of a `FrontRearParams` group (the copies
are the front and the rear tyres, by name; which comes first is not proved). A group holds: `SkidMarkParams` (an empty
structure, a head only), a `LatParams` (eight floats), two `LongParams` (six floats each), a `SmokeParams` (head only) and again
a `LatParams` and two `LongParams`. So there are two parameter sets for the marks (the first three) and two for the smoke (the
last three). Values of the Porsche 911 Carrera S (`VEH_1085007`), file order:

| Group, part | Floats |
|---|---|
| copy 1, marks, `LatParams` | 0.35, 0.1, 60, 50, 0.45, 23.5, 0, 0.2 |
| copy 1, marks, `LongParams` | 1.25, 0.075, 1, 50, 0.25, 0.2 and 0.15, 0.2, 0.8, 10, 0.1, 0.01 |
| copy 1, smoke, `LatParams` | 1, 0.04, 55, 50, 0.75, 23.5, 0, 1 |
| copy 1, smoke, `LongParams` | 0.1, 0.009, 0, 25, 1, 0.01 and 0.2, 0.1, 1, 150, 0.1, 0.1 |
| copy 2, marks, `LatParams` | 0.35, 0.1, 45, 50, 0.75, 7, 0, 0.2 |
| copy 2, marks, `LongParams` | 0.75, 0.075, 1, 50, 0.25, 0.2 and 0.15, 0.2, 0.8, 10, 0.1, 0.005 |
| copy 2, smoke, `LatParams` | 1, 0.04, 40, 50, 1, 7, 0, 1 |
| copy 2, smoke, `LongParams` | 0.25, 0.1, 1, 65, 5, 0.01 and 0.2, 0.1, 1, 150, 0.1, 0.1 |

The two copies differ in the third and fifth and sixth lateral numbers and in a few longitudinal ones, which fits front and rear
tyres of a rear-engined car (the rear lets go earlier, at 45 against 60, and its smoke and marks come on stronger). What each
number is (a slip angle at which marks start, a rate, a width, a response) is not known. The `w` of the wheel smoke binding
described in [chameleon-effects.md](chameleon-effects.md#wheel-effects-tyre-smoke) is most likely derived from these.
The rewrite's tyre smoke and skid marks are the host's own (from the slip of its vehicle library), so these numbers are
**not used**; the 2012 smoke look is the profile of the "Tire smoke variant" ([effect-variants.md](../modding/effect-variants.md)).

"Drift Tires Pro" ("Improves ability to enter and maintain large drifts. Emits colored tire smoke", string 2204083, bought in the
shop as a mod) is the only place where the game ties smoke to drifting: a coloured smoke on top of the tyre's smoke. The rewrite's
colourful smoke is the game's own setting ([tire-effects.md](../tire-effects.md#colourful-smoke)).

### Tyre noise: `TyreSoundParams` **[verified]** numbers, roles **[unconfirmed]**

`HAWAII/SURFACELIST.BNDL` holds one `Genesys.Gen.TyreSoundParams` object (`0x1B0CA7`) with five structures, all floats, by name
`Lateral`, `LatSlip`, `Longitudinal`, `LongSpinBraking` and `LongSpin`:

| Structure | Floats |
|---|---|
| `Lateral` | 0.4, 1.3, 1, 25, 20, 18, 80 |
| `LatSlip` | 1, 0.1, 40, 10, 0.05, 0.5 |
| `Longitudinal` | 0.8, 0.2, 0.6, 0.4, 50, 170 |
| `LongSpinBraking` | 1, 0.5, 1, 40, 0.3, 0.05 |
| `LongSpin` | 1, 0.2, 20, 2, 0.12 |

The skid sound graphs of `HAWAII/SURFACESOUNDS.BNDL` read the vehicle sound properties `LatSlipIntensityForSkidSound` ("The
lateral slip intensity as a [0,1] value"), `SpinIntensityForSkidSound`, `SpinBrakingIntensityForSkidSound` and `SkidLfo` ("The
current Skid LFO value") **[exe]**, import six samples of the surface sounds and play on the mixer channel "Player Skids"
(`MIXER_CHANNELS.BNDL`, `0x3F76D`). So the squeal is a layer of graphs fed by a 0 to 1 lateral slip intensity, a spin intensity
and a braking-spin intensity, with a low-frequency modulation; `TyreSoundParams` are most likely the curves from the wheel's slip
to those intensities. A drift raises the lateral intensity and nothing more. The rewrite's squeal is the 2005 game's skid graph
fed by its own wheel slip (`crates/nfsmw/src/audio/car/tyres.rs`); there is no mod call to change it, and the shared bundles'
skid samples were not matched to surfaces ([chameleon-shared-sounds.md](chameleon-shared-sounds.md)).

## Camera, post effects, haptics

**Camera [scan].** Every camera number of a car is in [chameleon-camera.md](chameleon-camera.md); none is a drift or a slip
field. The chase camera's `Yaw_spring_modification` (a speed range, a spring rate and a 0.05 s lead along the yaw rate) and
`Camera_Roll` (steering times a maximum of 4.7 degrees at full speed) are the only responses that depend on the car's turning,
and the rewrite's model (`libs/chameleon-camera`) applies both. In a slide the camera therefore follows the car's *heading*
(not its direction of travel, which it takes in the air only), so it turns with the nose and the car shows its side; the roll
follows the steering, which is opposite in a counter-steered drift. That is all the data supports; an extra "drift look" would
be invented.

**Post effects [scan].** `POSTFX.BNDL` (80 KB) and `GLOBALEFFECTS.BNDL` have no drift or slip reference. The speed effects of the
rewrite's nitrous variant (`[screen]` in [nitrous-variants.md](../modding/nitrous-variants.md)) follow the nitrous burn, as in the
2012 game; a drift only fills the tank.

**Haptics [scan].** No rumble or force-feedback resource or string names a drift. The host has pad rumble for its HUD
notifications (`hud/notify/output.rs`) but no mod call for it.

## What the rewrite does

In a 2012 car the mod does what [chameleon-drift.md](chameleon-drift.md) says: the sliding physics, the nitrous and
the "DRIFTING {distance}" line. The effects above are either not in the data (camera, post effects, haptics, a free-roam score),
or are the host's own responses to slip (smoke, marks, squeal) that the mod API cannot change. Nothing was added for them on
purpose: each would be an invention, not the 2012 game.

What a mod call would need for the rest, if wanted later (not built):

| Wanted | Missing call |
|---|---|
| a louder or different squeal in a drift, with the 2012 samples | a looping overlay sound with a gain the mod sets each tick (`bb.audio` 1.0 plays install banks once) |
| more smoke in a drift | a smoke rate scale the mod sets each tick (the tire smoke profile is a static file) |
| the Drift Attack score line in free roam | nothing missing in the API (`bb.hud` shows text); the rule is not known well enough, see above |
| a camera tweak | none missing; there is no data for it |
| pad rumble | a rumble request call |

## What is not known

- The units and rate of the Drift Attack score tick, the rounding step, the count in the tick, what a missed gate does, and the
  medal thresholds (the `TargetScore` getters read the event's data, which was not followed).
- What each `TyreVfxBehaviour` and `TyreSoundParams` number is, and which copy is the front.
- Who reads `IsDrifting` (the getter at `0x6DAFB0` calls a virtual function of the player state, slot `+0x6C`, whose
  implementations were not found) and whether it changes anything on screen beyond the nitrous message.
- Whether the HUD movie for the nitrous message does more than show the text (position, colour, fade); the movies are not read.

## Checks

The numbers above were read with `chameleon-bundle` and `chameleon-text` over the install (a throwaway scanner, not kept) and with
Ghidra on `NFS13.exe`. `cargo test -p chameleon-vehicle drift -- --ignored` (with `CHAMELEON_GAME_DIR`) checks the drift rule's
numbers on the 93 cars; nothing on this page has a test of its own because no code was built from it.
