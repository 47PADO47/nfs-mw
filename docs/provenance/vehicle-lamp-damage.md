# Named vehicle lamp damage provenance

Spec: [vehicle-lamp-damage](../specs/vehicle-lamp-damage.md).

The saved PC program identity was checked before bounded read-only queries:
`speed.exe`, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
No original process was launched or attached and no installation was modified.

The `EDamageLights` event vtable points to the deleting destructor `0x640600`
and name function; its destructor body `0x637E70` resolves the bound simable,
queries `IDamageableVehicle` using `0x620FD0`, obtains a named mask through
`VehicleFX::LookupID` (`0x73E540`), and calls interface slot eight. Disassembly
confirms the two arguments are the resolved mask and true; pseudocode alone
omits the second argument. The callback at `0x62B5B0` copies the static-data
name at byte four and current context handle. Its raw bytes were inspected
without creating a missing Ghidra function. The Lua binding at `0x62B5F0`
requires one name argument and binds the same current context.

`DamageLight` (`0x6B8910`) independently confirms the visual-admission virtual
call followed by mask OR/set or AND-not/clear. `IsLightDamaged` at `0x68C1D0`
checks destruction before the mask. The racer override at `0x68CCD0` checks
visual admission first. The raw 25-byte racer admission routine at `0x68CD40`
returns false when the frontend settings pointer exists and its visual-damage
byte is false; it otherwise returns true. The reference labels that option
but does not supply a complete implementation, so it is a naming cross-check,
not substituted proof. There is no unconditional player test in this routine.

Static PE comparison also hashed these exact routine ranges in a second,
different installed executable; all inspected callback, destructor, mutation,
query and gate ranges match byte-for-byte. This permits the stated bounded
cross-build comparison, not reuse of unrelated addresses without checking.

The CC0 dbalatoni13/nfsmw reference's generated `EDamageLights.hpp`,
`Interfaces/Simables/IDamageable.h`, `Physics/Behaviors/DamageVehicle.h`,
`DamageVehicle.cpp`, `DamageRacer.cpp`, and `World/VehicleFX.cpp/.h` supply
interface names, the complete symbolic mask table and repair semantics.
The selected PC's lookup loop and startup table initialization cross-check
the table's hash-based structure. The implementation must use symbolic names
and derive their hashes with the existing hash function.

A read-only recursive chunk/CARP census of both installed world files found
identical SHA-256
`7973374e53aa079cd4a8ec4011dd9aa11c680ca1868311ca4f4216fbd97b008f`:
73 event packages, 75 engines, one `EDamageLights` event. Its `CopCar` engine
has an aligned 16-byte record naming `LIGHT_COPS`. Event-list/action/state
references establish the producer described in the spec. The census does not
claim complete coverage of script/executable/cinematic producers or original
runtime visual output. No installed blobs or decompiler output are shipped.

In that package, `DZ_TOP`'s initial state accepts `DAMAGE_1` and starts
action `0x67808AE5`, whose start list sends `EProcessStimulus` to the
light-bar system with lowercase `trigger` and queue mode zero. `DZ_LEFT`
reaches the same action after `DAMAGE_1` then `DAMAGE_2`. The named-light
action `0x310A6CE2` first fires `EDamageLights`, then the light-bar fragment.
The root `EXPLOSION_DAMAGE` action invokes the same event list through an
undelayed nested list. These are installed reference relationships, not
inferred new thresholds. The collision producer and original zone/stimulus
rules are documented independently in [world sequencers](../specs/world-sequencers.md).

No body-zone-to-light damage threshold was inferred. Unknown default names,
unrelated unexamined call sites, partial-side surface rendering and original
live optical agreement remain separate limits. Existing NOTICE credits the
CC0 reference; this spec and any implementation describe independently
observed behavior rather than copying original code.

The current normal-game host admits visual damage and has no corresponding
frontend disable-option adapter. The named event implementation follows that
enabled path for every vehicle; it does not invent a player exemption. A future
visual-damage toggle must also gate the racer mutation/query as specified.
