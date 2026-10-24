# Garage customization records

Sources: the CC0 `dbalatoni13/nfsmw` reference, `VehicleDB.hpp`, inspected
2026-10-23. Reference declarations establish layout, not observed gameplay.
The independent codec below preserves fields it does not change.

A customization record occupies 408 bytes in the career stable. Each car
record selects one of 75 records by its customization index.

| Offset | Field |
| --- | --- |
| 0 | 139 signed little-endian 16-bit installed-part indices |
| 278 | two padding bytes |
| 280 | seven signed 32-bit performance stages |
| 308 | signed 32-bit Junkman bit mask |
| 312 | three profiles of seven little-endian 32-bit tuning values |
| 396 | signed 32-bit active tuning profile |
| 400 | signed 32-bit preset identity |
| 404 | one-byte record handle |
| 405 | three remaining bytes |

Part indices refer to the current installation's part database. Negative one
means no explicit part. They must be resolved against that installation;
hashes must never be truncated into this field. The seven performance fields
are tires, brakes, suspension, transmission, engine, induction and nitrous.
The seven tuning fields are steering, handling, brake balance, ride height,
aerodynamics, nitrous and induction. Slider positions map to normalized values
by division by five. An existing record retains its preset, handle and padding.

The codec accepts all bytes for lossless round trips. Consumers validate stages,
part compatibility, tuning finiteness/range and profile index before applying
them. Creating a new record uses absent parts and neutral tuning. Changing
one car must not modify another car's record.

Custom RGB values and future mod fields have no verified native record encoding.
The remake may retain them in a versioned extension tied to the profile/car
identity. Such extensions must not overwrite unknown original record bytes.

The native profile replacement is the purchase commit point. A write-ahead
journal holds old/new extension bytes and full native-profile digests. Recovery
before every profile read or write selects the extension matching that commit.
The customization-record digest alone is insufficient: an RGB purchase can
change cash without changing native customization bytes. A failed recovery
blocks further profile writes until it can complete. The record digest still
guards later extension recall after native parts change outside the remake.

Validation: synthetic byte round trips, field offsets, invalid record indices,
and preservation of neighboring records. Matched original-game save editing
and runtime comparison remain separate checks.
