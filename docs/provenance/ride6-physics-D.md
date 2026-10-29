# Provenance: RIDE 6, the rider in a fall (series D)

Covers [ride6-rider-fall](../specs/ride6-rider-fall.md). Earlier rounds: [ride6-physics.md](ride6-physics.md),
[A](ride6-physics-A.md), [B](ride6-physics-B.md), [C](ride6-physics-C.md).

## What was used

- The user's own install of RIDE 6 (Milestone, Unreal Engine 5.6.1, build `5.6.1-1697004`), read-only. No game file, asset or
  decompiler output is in the repository; the spec states algorithms and numbers in our own words.
- The shipping executable `ride6-Win64-Shipping.exe`, decompiled in a private headless Ghidra copy of the analysed project (a
  scratch folder, copied first; the listing holds no instructions for most addresses, so the functions were found from the
  executable's own tables and decompiled on demand). Functions were found by (a) the **native function tables** of the
  Unreal classes (the pairs of a function's name string and its `exec` thunk, located by scanning the executable's read-only data for
  the name's address), (b) the displacement scan of the class dump's field offsets (`RagdollVelocity*` at 0x46c to 0x488 of the
  rider component) mapped to functions by the exception tables, (c) call sites (a scan of the code section for relative calls),
  (d) the vtable of the input handler. Addresses are virtual (`0x14` + image-relative).
- The Dumper-7 class dump of the same build (`ignitiongameplay`, `ignitionphysics`: names and offsets of fields, no code).
- The data assets, read with the scratch package reader: `RiderGP_PhysicsAsset` (bodies, masses, capsules, constraints),
  `RiderBP` (the ragdoll component's `StartFallConfigurations`, the rider component, the skeletal mesh component's collision
  responses and material), the physical materials `PM_RiderRagdoll_CB`, `PM_RiderRagdoll`, `PM_T_Tarmac`, `PM_T_Grass`,
  `PM_T_Gravel`, the curve `Ragdoll_strength_Race`, the rider animation blueprint's variables and the list of its ragdoll clips, the
  KTM's input handler (`FallHighsideLeanLimit` 70, `FallSteerComponent.SteerRadius` 25).
- Hand-decoded constants from the read-only data section (km/h to cm/s 100/3.6 = 27.7778 at `0x14a3e8050`; the degrees to radians factor
  at `0x148fbad80`; the zero vector `0x14bc01348`, the up vector `0x14bc01378`).

## Functions read

| Topic | Functions | How verified |
|---|---|---|
| The handler starts a fall | the exec of `StartFall` `0x14509dd00`: sets the fallen byte (`+0x2d8`), records the vehicle's transform, queues the deferred call `0x1450c9df0` (a call of a vtable slot `0x488` of its object), fires the event at `+0xee8` | read; the event's field offset is the dump's `BikeInputFallenEvent` |
| The fall tests (kinds) | `0x1450ca0a0` (earlier rounds) | [ride6-chassis-round6.md](../specs/ride6-chassis-round6.md) |
| The rider component hears the fall | `OnBikeFallen` of the rider component `0x145b4db30`: sets the pending byte (`+0x4c8`), compares the fall type's name with the wide strings `HighSide` (`0x149e3f878`) and `LowSide` (`0x149e3f870`) and stores 2, 3 or 1 at `+0x4c9`; its sibling `0x145b4daa0` is the exec's callee (a guard on flags and a virtual call) | read; the strings read from the data section |
| The pending ragdoll starts | `0x145b4fdf0`: clears the pending byte, takes the oldest entry of a ring buffer (`+0x560` array of `0x38`-byte entries, count at `+0x568`, cursor at `+0x570`), calls the velocity function, multiplies by `RagdollVelocityMultiplier` (`+0x484`) and calls `EnableSimulation` (and, with a character-physics component present, the path `0x145b2dd20`) | read; the fields match the dump's names and offsets |
| The launch velocity | `0x145b3ea00`: `rand()`, angle between the two `RagdollVelocityRandomRotationRange` numbers (`+0x470`, `+0x478`, degrees to radians), sign by the lean against `+0x480`, a rotation about the up vector; the damper offsets (`+0x490` front, `+0x4a8` rear) chosen by the larger compression against `+0x488` and moved by the bike's transform | read; every default is zero (below) |
| The component's defaults | the constructors `0x145b2b0a0` and `0x145b2be10`: `+0x46c` = 2, `+0x470`/`+0x478` = 0, `+0x480` = 0.5, `+0x484` = 1.0, `+0x488` = 0, both offsets the zero vector | read; the blueprint's component properties override none of them |
| Ragdoll start | `EnableSimulation` exec `0x145a35040`, body `0x145b43720`: marks the simulation on, assigns the mesh, the root detach (`bDetachRoot`), calls the velocity functions | read |
| Launch velocity with a configuration | `0x145c2db30`: `v' = v + R a + w + R (m (.) R^-1 v)`, the angular velocity times `FallAngularVelocityFactor`, the whole times 100/3.6; without a configuration `0x145c2d850` adds the velocity (and `StartFallAdditionalVelocity`, zero) to every body | read; the constant read from the data section |
| Character-physics path | `ActivateFromBP` exec `0x1459d7fa0`, body `0x145b2e250` / `0x145b2dd20` | exec only |

Other names found in the tables and not read: the `RaiseBike` component's `OnBikeFallen` (a third class with the same event),
the simplified-ragdoll component.

## Layout facts that cost time

- The rider component's exec thunk for `OnBikeFallen` and the one of the raise-bike component are in neighbouring tables; the
  one with the rider component's fields (`+0x4c8`, `+0x4c9`) is `0x145b4db30`.
- A `FallConfiguration`'s three vectors are in km/h and its velocities in the *rider mesh's* frame; the factor 100/3.6 is applied
  once at the end.

## Data read

The numbers of the spec: `RiderGP_PhysicsAsset` (11 bodies: masses and capsules; 10 constraints: motions and limits),
`RiderBP` (`StartFallConfigurations`: high side `(0, 0, 6)` / `(0.05, 0, -0.28)` / 0 / 0.5, low side 0 / `(0.05, 0, 0.32)` / 0 / 0.5),
the physical materials (rider 0.62 / 0 / sleep 5, 1; road default 0.7), the rider mesh's collision responses (vehicle, pawn, physics
body, world dynamic ignored).

## Not verified

The name broadcast for each kind of fall (the codes and the names were matched by their meaning, not traced); that the buffer's
oldest entry is exactly two frames old; which of the vehicle's mass the rider takes with him; the physical-animation blend (the
strengths and the clip choice); the get-up sequence. No statement here was tested in the running game (the game must not be launched).
