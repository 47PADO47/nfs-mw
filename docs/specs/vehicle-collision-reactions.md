# Vehicle collision reaction profiles

Sources: original PC 1.3 selection `0x68BEF0`, goal selection `0x422480`,
and the CC0 reference revision recorded in
[pc-object-contact.md](pc-object-contact.md). Private inspection stays outside
the repository. This specification precedes the implementation.

`pvehicle.aivehicle` links to inherited AI attributes. `PlayerCollisionsDefault`
is a reference to `collisionreactions`. Each `PlayerCollisions` entry is a
16-byte `AICollisionReactionRecord`: a little-endian goal key at 0, followed
by a 12-byte reference (class at 4, collection at 8, reserved at 12).
Select the first entry matching the current goal; otherwise use the default.
If that first matching reference is invalid, fall back to the default. Do not
continue searching later duplicate goal keys. A missing or invalid default leaves
no reaction profile.
Goal keys are opaque `UCrc32` values, never guessed from model names.

A profile contains `FRONT_REACTION`, `FRONTSIDE_REACTION`, `REAR_REACTION`,
`REARSIDE_REACTION`. Each is 24 bytes: six little-endian floats for elasticity,
roll height, weight bias, mass scale, stun speed and stun time, respectively.
Read the profile through its inherited attributes, including van, truck and
individual police variants. Do not replace it with a category-wide preset.

Apply a profile to an AI car only when the other driver is human or a racer.
Choose front when `(contact point - body position) dot forward > 0`;
otherwise choose rear. Choose side when `abs(normal dot forward) < 0.707`;
otherwise choose the front/rear end record. Exact boundary ties select the end
record, and a point exactly on the transverse centre plane selects rear.

Start from ordinary object material and a temporary copy of the body's centre
of gravity. Add profile elasticity and clamp to [0,1]. A positive mass scale
multiplies **both** temporary mass and temporary inertia; zero/negative leaves
both unchanged. Add roll height to local centre y and weight bias to local z.
Select these parameters before penetration correction and move their world
centre with the body during correction. Never change integration mass, tensor,
suspension settings or stored centre of gravity.

Before vehicle-specific adjustments, initialize each contact moment's immobility
from its own source `IMMOBILE_OBJECT_COLLISIONS` flag or attachment state. A
partner's source immobility never clears that initial state. The base modifier
assigns an attached body's breakaway impulse only when its partner is mobile;
against an immobile partner it retains immobility with zero breakaway impulse.
This eligibility concerns source state, not a temporary `NO_CAR_EFFECT` moment
override. When both source bodies are immobile, both contact moments remain
immobile. Vehicle immobility remains an object-contact override and
must not freeze integration or ground/world response. Installed-source and live
construction checks on 2026-10-20 identify vehicle profiles with this flag,
including carriers/tankers; omitting it changes their resistance to loose props.

The grounded player adjustment applies only to the actual player when both
vehicles have more than two wheels on the ground: temporary local centre y is
`point.y - position.y - 0.125`. It is independent of the steering input device.
AI against AI uses ordinary material unless the other driver is a racer.

Runtime free-roaming traffic and ordinary chasing/patrol cops use their default
profile. Active pit, ram (box-in and rolling-block), pull-over and static-roadblock
states select their authored goal override. A failed or ended goal clears that
selection. This does not implement the complete original roadblock state machine.
Stun fields are parsed for audit, but their original AI state effects remain open.

Installation validation must resolve every default and goal reference and all
four records, not just a small set of vehicle models. Runtime tests check
profile selection, partner eligibility, centre/mass/inertia changes, and unchanged
stored body parameters. Numerical and frame-rate checks do not establish full
live original-game collision parity.


The 2026-10-21 isolated PC selector audit exercises the exact binary float
threshold and its adjacent representable values on both normal signs, the
front/rear point seam and rotated bodies. Reaction selection uses the actual
contact plane before penetration correction; damage-zone classification is a
separate callback and must not substitute for this four-way selection.
