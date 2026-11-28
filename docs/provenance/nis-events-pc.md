# NIS event handlers on the PC (`crates/nfsmw/src/scenes/world/nis`, `crates/nfsmw-data/src/world/scene_cars.rs`)

- **Spec:** [specs/nis-events-3.md](../specs/nis-events-3.md). It replaces the inferred event meanings of
  [specs/nis-events.md](../specs/nis-events.md) section 2.

## Method

- Read in Ghidra (headless, read-only) and with a disassembler in `speed.exe` 1.3 of the unmodded install (SHA-256
  `80774c2e…`), 2026-11-18. The program's function names come from a private symbol map; decompiler output was kept in a
  private folder and not copied. The CC0 decomp (dbalatoni13/nfsmw) gave the member names of `SuspensionSpline`,
  `EngineSpline`, `DrawCar`, `CarRenderConn`, `IVehicle`, `IDamageable` and `INISCarControl`; the PC code read matches
  them where both exist.
- Live: the unmodded game was run hidden and muted through the career prologue with hooks on every event type's
  make-event callback and on the detach and reattach handlers, logging the animation clock, the target car (name,
  position, animating flag) and the event fields, and sampling the targeted cars every 0.1 s of scene time (1 run, the
  six prologue scenes).

## Addresses read (`speed.exe` 1.3)

- Event pool and context: the make-event callbacks allocate from the pool at `0x00920390` and read the target handle
  at `0x0091F1B8`; handles are looked up in the sorted handle table at `0x0092FEB4` (count `0x0092FEBC`).
- Make-event callbacks: `ENISAeroDynamics` `0x0062DA50` (writes the aero global `0x00937800`), `ENISDetach` `0x0062DD40`,
  `ENISFakeFar` `0x0062DEC0`, `ENISMotionBlur` `0x0062E320` (writes `0x008F9B28`), `ENISNukeSmack` `0x0062E3F0`,
  `ENISPlayEffect` `0x0062E6E0`, `ENISReattach` `0x0062E920`, `ENISRoadNoise` `0x0062E9F0` (writes `0x009B09B0`),
  `ENISTimeOfDay` `0x0062ECB0`, `ENISDetail` `0x0062DE10`, `ECameraPhotoFinish` `0x0062B0A0`, `ECellCall` `0x0062B2F0`,
  `EDDaySpeech` `0x0062B6B0`; the others call their constructors below.
- Constructors that act: `ENISBrakelock` `0x00638FB0`, `ENISBurnout` `0x00639190`, `ENISCarDamageReset` `0x00639350`,
  `ENISCarPitch` `0x006394D0`, `ENISCarRoll` `0x006396B0`, `ENISCarShake` `0x00639890`, `ENISLights` `0x00639D00`,
  `ENISNeutralRev` `0x00639F20`, `ENISNitro` `0x0063A140`, `ENISSteering` `0x0063A3F0`, `ENISRain` `0x006233D0`,
  `ENISStopEffects` `0x006235D0`, `ENISOverlayMessage` `0x00623060`.
- Destructors that act: `ENISDetach` `0x00639C70`, `ENISReattach` `0x0063A320`, `ECameraPhotoFinish` `0x00640410`,
  `ECellCall` `0x0062B180`, `EDDaySpeech` `0x0071DCE0`. The other event destructors only reset the vtable.
- Interfaces: `IVehicle` handle `0x004040E0` (glare on/off at vtable `+0xA0`/`+0xA4`, `SetAnimating` at `+0xC0`),
  `IDamageable` `0x00405000` (`ResetDamage` at `+0x10`), `INISCarControl` `0x00445650`, `INISCarEngine` `0x00620460`.
- Car behaviours: `SuspensionSpline::NISCarTweaks` `0x00690EA0`, `SetNISPosition` `0x00691110`, `OnTaskSimulate`
  `0x006B34A0`; `DrawCar::OnService` `0x006B6FB0` (the NIS pitch, roll and shake into the render packet).
- Motion blur consumer `0x006DBB20`; scene release `0x006F7520`; `NISActivity::UpdatePlaying` `0x006EA750`;
  `CAnimScene::UpdateTime` `0x004507C0`; `EmitterSystem::KillEverything` `0x0050AF20` and `RefreshWorldEffects`
  `0x0050D170`.
- Camera: the mover update `0x00482EC0` (spaces and key bytes 6 to 9), the key setup `0x00482160` (overlay swap), the
  overlay table at `0x008EDEEC` read by `0x0046E9C0`, show/hide `0x0046EA50`/`0x0046EAE0`, the world and car constraints
  `0x00470FE0` and `0x0047C6F0`, the player car frame `0x0047EED0`.
- Special car lists: `NISActivity::PrepareVehicles` `0x006F2F60` with the nine traffic mixes of eight vehicle keys at
  `0x008EC0F0` (0x20 bytes each). The keys were named by hashing the install's vehicle names with the vault hash.

## Implemented by

- `crates/nfsmw/src/scenes/world/nis/`: `events.rs` (decoding, car events per channel), `car_events.rs` (the pulses, the
  shake, the lock, the lights, the detach state), `coast.rs` (the detached car), `car_state.rs` (lights and repairs on the
  real car), `car_world.rs` (poses, locked wheels), `handlers.rs` (motion blur, stop effects, the logged events),
  `player.rs` (a channel with no car fires nothing), `scene_list.rs` and `car_world.rs::link_cop` (the list vehicles).
- `crates/nfsmw/src/scenes/world/ai/nis_vehicles.rs` (making a list vehicle), `ai/car/accessors.rs` and `drive/nis_pose.rs`
  (the scene's lights).
- `crates/nfsmw-data/src/world/scene_cars.rs` (the lists) and `moment.rs` (the list codes from the candidate table).

## Checked against the game by

- Unit tests: `car_events_tests.rs`, `coast.rs`, `car_memory.rs`, `events_tests.rs`, `handlers_tests.rs`, `scene_cars.rs`.
- Install test: `crates/nfsmw-data/src/world/moment_tests.rs` (`installed_scenes_name_their_special_car_lists`).
- The live event log of the original against the rewrite's `nis-report` (event, channel, time), and frame captures
  against the original's pictures; the results are in the private research notes.

## Known differences

- Detached cars follow their channel until it is cut, then coast; no collisions, suspension or NIS input. The original's
  cars run their physics, crash and brake.
- Nitrous, neutral revs, aerodynamics, rain, road noise, the photo-finish pulse and the D-Day speech are not acted on.
- Camera key bytes 6 to 9, hybrid space and overlay values 2 and 4 are not applied.
- `ENISPlayEffect` plays its effect in the rewrite (a stand-in); the original does nothing with it.

## Opening intersection and ramp

The stock PC scene trace releases the minivan and a patrol car at scene time 26.61 s and the stationary sedan at
27.01 s. Between 26.6 and 28 s the minivan advances past the sedan. Its recorded height varies by 0.58 m.
The remake's former channel-following behavior stopped the minivan short and held its height constant.
Replaying the opening with scene-owned cars on the vehicle solver produced a 0.49 m height range. Over 14
paired samples the median distance in the two available position axes was 1.2 m. One original coordinate
was missing. The release and reattach handlers are described in the event spec.

The later ramp pile-up remains an imperfect match. In particular, body contact alone does not reconstruct
the authored prop effect or establish its debris, smoke, or camera response.
