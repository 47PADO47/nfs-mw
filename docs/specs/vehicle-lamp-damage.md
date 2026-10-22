# Named vehicle lamp damage

Sources and verification boundary: [provenance](../provenance/vehicle-lamp-damage.md).

## Event and target

**[decomp, PC checked]** `EDamageLights`, event ID `0x4FD8F6DC`, has an
eight-byte minimum static payload: repeated event ID at byte zero and an
AttribSys/lookup2 lamp-name hash at byte four. Trailing alignment padding is
permitted; the inspected installed record is 16 bytes. The name is already a
hash, not a package string-table tag. Do not resolve it through package strings
or default an absent/unknown name to all lamps.

The event constructor binds the current dynamic-context simable handle.
Dispatch resolves that handle, obtains `IDamageableVehicle`, looks up the
named `VehicleFX` mask, and calls `DamageLight(mask, true)`. Missing target or
interface does nothing. An unrecognized name is an original assertion/error
path; the remake must report it explicitly without damaging guessed lamps.
The event supplies no world position, body zone, severity, target override or
clear flag. Preserve context through nested event lists.

## Names and mutation

Use one symbolic resolver for the original names. Each row is an original
`LIGHT_` name and its mask; these are separate from model marker names.

| Name | Mask |
|---|---:|
| `LIGHT_LHEAD`, `LIGHT_RHEAD`, `LIGHT_CHEAD` | 1, 2, 4 |
| `LIGHT_LBRAKE`, `LIGHT_RBRAKE`, `LIGHT_CBRAKE` | 8, 16, 32 |
| `LIGHT_LREVERSE`, `LIGHT_RREVERSE` | 64, 128 |
| `LIGHT_LRSIGNAL`, `LIGHT_RRSIGNAL` | 256, 512 |
| `LIGHT_LFSIGNAL`, `LIGHT_RFSIGNAL` | 1024, 2048 |
| `LIGHT_COPRED`, `LIGHT_COPBLUE`, `LIGHT_COPWHITE` | 4096, 8192, 16384 |
| `LIGHT_COPS` | 28672 |
| `LIGHT_LSIGNAL`, `LIGHT_RSIGNAL` | 1280, 2560 |
| `LIGHT_HEADLIGHTS`, `LIGHT_REVERSE`, `LIGHT_BRAKELIGHTS` | 7, 192, 56 |

`LIGHT_COPS` hashes to `0x5B667470`; the grouped names are masks, not additional
independent lamps. Names without original mapped marker positions do not gain
invented geometry. The empty-name sentinel terminates lookup and is not a
request to damage every lamp.

When visual damage is admitted, `DamageLight(mask, true)` ORs the mask into
independent accumulated lamp damage. A repeat is idempotent; a different mask
accumulates. The interface's `false` argument clears only the supplied bits,
although `EDamageLights` always passes true. Full repair clears all individual
bits. Keep the destruction flag independent: a destroyed vehicle still reports
all lamps broken even if its individual mask was empty or partially cleared.

The base vehicle admits visual damage. The racer override consults the
frontend visual-damage option; when it disallows visuals, neither individual
light mutation nor its broken-light query should expose damage, including the
query's destruction fallback. This is not an unconditional player exception.
Do not equate driver/player classification with that setting. Exact settings
adapter integration must preserve this distinction.

## Installed producer and scope

The inspected world catalog has one `EDamageLights` record, in the `CopCar`
engine. It names `LIGHT_COPS`. Its action starts on a system's response to
lowercase `trigger` (`0x3C779B79`); the same state also accepts
`EXPLOSION_DAMAGE` with an empty action. Keep the authored state machine and
action scheduling. Do not replace the trigger with a new panel severity
threshold.

The installed collision path is concrete: `DZ_TOP`'s initial state responds
to `DAMAGE_1` with an action that starts `EProcessStimulus` targeting the
light-bar system's `trigger`. `DZ_LEFT` reaches the same action on `DAMAGE_2`
after its first visual-damage transition. That process-stimulus event has
queue mode zero and starts the named-light action, which damages `LIGHT_COPS`
before spawning the light-bar fragment. The event's mask therefore belongs
to authored light-bar detachment, rather than a new generalized zone rule.

The engine's root system also responds to `EXPLOSION_DAMAGE` with a list
that invokes this light-bar list using undelayed `EFireEventList`; retain its
dynamic owner context and list order. A direct root destruction broadcast is
not a substitute for either authored path.

This catalog census establishes a missing authored cop-light producer. It
does not establish arbitrary headlamp/tail-lamp breakage on every crash, nor
prove that all executable/Lua/cinematic callers are covered. General named
event support must not be presented as verified crash triggers for every lamp.

## Acceptance

Decoder coverage must require the name word, accept trailing padding, and keep
the name hash intact. Test the symbolic resolver's individual and grouped
masks; unknown names must not mutate damage. Exercise the installed `CopCar`
system and its real collision-to-zone-to-action path into the production
vehicle adapter, requiring only the cop bits to change, repeated dispatch to
remain stable, and repair to clear them. A top impact that reaches the first
authored visual tier provides an installed positive case; another zone that
does not reach this action provides a control. Cover the root explosion's
nested list separately. The tests must demonstrate no accidental head/brake
damage and respect the source action's timing/state transition.

The resulting mask must reach player, AI and independently posed trailer flare
adapters, historical trail copies, emergency sound/emitters where their
consumer checks the cop group, and the supported surface-state selection.
Use a synthetic named-head fixture only to prove decoder/adapter behavior;
label it separately from the installed cop producer. Preserve the existing
independent per-marker flare rules and the documented partial-side brake
surface limitation in [player car lights](player-car-lights.md).
