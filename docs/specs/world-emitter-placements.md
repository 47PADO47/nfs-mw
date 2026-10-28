# World emitter placements

The streamed world contains `EmitterLibrary` chunks (`0x0003BC00`). The
layout and loading rules below come from the CC0 reference's
`Ecstasy/EmitterSystem.h` and `EmitterSystem.cpp`. They are also checked
against the PC track stream. No game data is included here.
See [provenance](../provenance/world-emitter-placements.md).

The 16-byte-aligned payload starts with four little-endian 32-bit words:
endian state, version, library count and section number. Version 5 is present
in the inspected PC stream. Each library has an 80-byte record followed by
its 48-byte trigger records. Its group key is at offset 0, section at 8,
trigger count at 10, and row-vector local-to-world matrix at 16. The matrix's
translation is its fourth row. Trigger records contain radius at 12, world
position at 16, reset time at 32 and probability at 36.

The original associates a group with each library and updates it as sections
stream. Nonzero ordinary sections can instantiate a group when its position
is near a player view; the reference uses its authored `FarClip` plus 20 and
a view test. Triggered effects also have trigger-specific activation rules.
Group keys resolve to the installed `emittergroup` class. The PC service and
trigger predicates are described below. Global particle-pool scheduling and
matched original-game images remain separate validation work.

The reader must bound counts and reject truncated data. It must preserve
trigger records rather than treating every placement as always active. A
parsed placement alone is not evidence that its effect appears in gameplay.

## Bounded allocation

Use deterministic nearest-first scenery admission, at most 64 scenery placements,
16 emitters per group. Particle capacity starts at 128 and grows only when the
authored effective rate and maximum life require more, up to the engine hard
limit of 1024. One-shot on-cycles bound the emission window; invalid fields keep
the base capacity. Two slots cover the spawn-before-age boundary. Activity-icon placements
have their own visibility policy and do not compete for the scenery admission
budget. Missing groups and textures produce no substitute effects. The world
simulation clock controls particles and trigger cooldowns. These bounds are not
original pool parity: PC shares 200 groups, 500 emitters and 1024 particles
across effects and evicts lower-priority groups when its pools fill.

## PC service and trigger rules

[decomp] Retail PC services every ordinary nonzero section library, including
libraries containing trigger records. Trigger metadata must not suppress the
ordinary effect. Admission uses distance strictly below FarClip + 20 and a
normalized view-direction dot product strictly above 0.55. Particle spawning
uses FarClip and 0.7 instead, while existing particles continue advancing.
Section 0 is a template; section 0xa28 is always admitted. Multiple local views
are tested independently.

Trigger evaluation uses each player's rigid-body position predicted 0.15 seconds
along its velocity, in world-render coordinates. An armed trigger activates only
on entering its strict-radius sphere. Probability zero never fires; 65535 always
fires; intermediate values fire when an integer random draw in [0,65535) exceeds
the stored value. This is a threshold, not a percentage. A failed roll does not
reroll while the player remains inside. An activated trigger waits for its
library group to disappear, then waits strictly longer than reset_time before
rearming. It must observe an exit before another entry. Trigger-created groups
use authored emitter control; the trigger flag does not force one-shot behavior.
Ordinary service runs before trigger dispatch; a trigger never duplicates an
already-running library group.

The remake retains deterministic bounded allocation rather than claiming the
original global pool's priority/eviction behavior across every particle system.

The remake may upload effects several times for one simulation instant. Trigger
queries and cooldown transitions run once per changed simulation clock, including
initial load. A repeated upload with unchanged camera views must not retire a
newly triggered group before its first particle step. Camera changes still update
ordinary admission while paused; stream retirement and activity-icon changes
remain immediate. These are renderer integration rules, not a claim about the
original application's pause UI.

## Installed capacity census

The inspected stock stream has 1535 placements and 16 distinct emitter groups;
the largest group has three emitters. The 16-emitter guard does not truncate any
of those groups. Sampling every placement origin and 16 horizontal camera yaws
against all stream placements found at most 54 admitted ordinary placements.
This is a bounded spatial census, not proof for every camera or split-screen pair.

The steamjet emitter uses 100 particles/second and two-second life without an
on-cycle or alpha retirement. A 120 Hz installed regression reaches more than
128 live particles and compares against the old cap. The new capacity is 202.
The one-shot blackbird and leaf emitters retain 128 because their respective
one- and six-second emission windows bound the population. No generic particle
spawn timing or shared global-pool arbitration changes in this repair.
