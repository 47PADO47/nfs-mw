# Native impact parity

The original collision solver and installed attributes define this work. No
replacement physics solver or family-specific mass multiplier is introduced.
Dynamic scenery retains the source mass, inertia tensor scale, breakaway impulse,
immobility, collision flags and primitive order. Police contacts additionally
use the live original goal's temporary collision profile; it must never rewrite
the stored physical body mass.

## World contact order

The native object task integrates free scenery and resolves object pairs before
vehicle contacts. Its end-frame ground support, drag and sleep task runs after
those contacts. Newly detached bodies therefore receive same-tick ground support
without a second integration. Spatial lookup bounds are refreshed after each
phase. This preserves the original separation of these tasks rather than moving
ground response ahead of every vehicle contact.

## Impact consumers

Physical velocity change and the original collision notification are distinct.
An immobile contact can report a normal impulse to sound, damage and camera
consumers without moving the car. Visual damage uses the original callback's
normal impulse and stored body mass, excluding restitution and friction.

Related contracts: [object contacts](pc-object-contact.md),
[ground support](ground-contact-response.md), [vehicle damage](vehicle-damage.md),
[police destruction](cop-destruction-contacts.md),
[NPC chassis contact state](npc-chassis-contact-state.md),
[camera shake](contact-camera-shake.md) and
[petrol explosions](petrol-station-explosion.md).

## Verification limits

Installed-data construction coverage checks that each available prop receives
its authored attributes. It does not establish identical impact trajectories.
Targeted production-path regressions cover parked cars, toll booths, police
impacts and damage; they do not replace comparison runs in the original game.
Jointed scenery constraints and original simple-physics/trigger semantics need
separate verification. Full city-wide gameplay and visual/audio parity remain
open until matching original-game and remake playtests cover them.
