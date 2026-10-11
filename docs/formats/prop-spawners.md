# Placed physics objects

Sources: the user's PC 1.3 files, privately inspected executable (SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`), and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0, reference only),
`Physics/SmokeableInfo.hpp` and `World/Scenery.hpp`, revision
`80993f35224c61d2f1b135beba76c6f8234c7940`.

## Spawner packs

**[verified, decomp]** Streamed sections contain chunk `0x00034027`. Align the
payload to an absolute 16-byte file address, then read a 16-byte header followed
by 64-byte records. The signed 16-bit count is at header offset `0x0c`.
Offset `0x0e` is a runtime initialization byte. The words at `0x08` and `0x0a`
are an internal section identifier and first object ID; neither is the streaming
section number. Do not infer the scenery association from them. The first eight
bytes are runtime list storage. Reject negative counts and incomplete records.

| Record offset | Type | Meaning |
|---|---|---|
| `0x00` | f32[4] | Orientation quaternion x, y, z, w in physics axes |
| `0x10` | f32[4] | Position x, y, z, padding; physics axes |
| `0x20` | u32 | Model name key |
| `0x24` | u32 | Collision bounds name key |
| `0x28` | u32 | `smackable` attribute collection key |
| `0x2c` | u32 | Global scenery override index |
| `0x30` | u32 | Unique object ID |
| `0x34` | u32 | Exclude flags |
| `0x38` | u32 | Runtime model pointer; ignored |
| `0x3c` | u32 | Padding; ignored |

Physics positions become renderer positions as `(z, -x, y)`. Orientations must
use the same reflected change of basis. Asset readers leave the values in their
source coordinate system.

## Scenery association

**[verified, decomp]** Each scenery section's `0x00034106` child has consecutive
4-byte records: u16 global override index at 0 and u16 local instance index at 2.
No alignment prefix. Join the spawner's override index to these hooks, then use
that instance's normal scenery info and LODs. Model names are truncated in the
scenery table; matching those names to physics attributes loses information.

The global `0x00034108` override table separately stores 6-byte records: signed
section number, signed instance number, unsigned exclude flags. Reading it is
unnecessary for a resident section that already has its hooks.

## Install measurements

**[verified]** On 2026-10-11 the L2RA stream contained 366 packs, 13,484 spawners
and 22,340 hooks. Every spawner had a unique ID, a hook in its containing streamed
section and a resolvable `smackable` collection. Every position exactly matched
the associated scenery position after the axis conversion. 13,479 had a matching
track bounds key; three `lightwoodenpost` and two `wood_slat_fence` records did
not. Missing bounds must be reported/skipped explicitly, not assigned a guessed
shape or another object's dynamics.

These observations establish the mapping and serialized layout. They do not
establish runtime destruction, hierarchy replacement, respawn or cargo behavior.
