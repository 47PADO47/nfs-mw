# Event sequences

Sources: the user's PC 1.3 install, executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`, and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0 reference,
revision `80993f35224c61d2f1b135beba76c6f8234c7940`), `CARP.h`, `UGroup.hpp`,
`Event.h`, `EventSequencer.cpp`, and generated event headers. These are format
facts, not copied code. Measurements below are from the install.

## Container

The world file `TRACKS/L2RA.BUN` has 73 leaf chunks `0x0003B811`, inside
`0x8003B810`, containing 75 engines. Align the leaf payload's absolute address to 16 bytes, then skip
the 16-byte runtime header. What follows is the embedded CARP group format
described in [collision.md](collision.md). Its root tag is `CARP`.

All integers and floats in record payloads are little endian. References stored
as four-byte values are **CARP record tags**, not offsets or host pointers. The
original loader resolves these tags after deserializing the group. A reader can
retain the tags and use a lookup table instead. A zero tag means no reference.
Do not assume the record's `count` means an array length: here it usually stores
the name's AttribSys/lookup2 hash. Counts inside the payload determine lengths.

The high two bytes of a tag identify the record type; the low two identify its
index. The tag `eE/0` is therefore `0x65450000`. A group also contains the `Strs`
string table; this is useful for inspection but is not needed to execute hashes.

## Records

| Type | Header | Trailing arrays |
|---|---|---|
| `iN`, `cS` | NUL-terminated UTF-8 string | none |
| `eE` engine | name-string tag, system count | system name hashes, then system tags |
| `eS` system | initial state hash, default state hash, state count | state hashes, then state tags |
| `eT` state | stimulus count, filter tag | stimulus hashes, then pairs of end-state hash and action tag |
| `eA` action | action name hash, tag count, time count, duration float | event-tag hashes, time floats, then event-list tags for all tags followed by all times |
| `el` event list | event count and 12 padding bytes | 16-byte event descriptors, then static data |

An event descriptor contains event-ID hash, static-data size in bytes,
static-data offset **relative to this descriptor**, and padding. Static data
begins with the event ID again. Its declared size can include alignment padding.
The next descriptor is exactly 16 bytes later, irrespective of data size.
Event order is significant. Validate every array, string terminator, reference,
time, descriptor range and the repeated event ID before exposing a sequence.

Filters contain query/expression bytecode. Keep their identity explicit;
unimplemented filters must not silently accept a stimulus. The log and car
carrier sequencers have no filters. Timed event lists may share one record.

## Vehicle fragment events

`ESpawnFragment` has event ID `0x2554C48A`. Its 48-byte static data holds the ID,
part-name hash, collision-node hash, smackable-attribute hash, joint hash,
allow-duplicates flag, inherited-velocity multiplier, hide-parent flag,
hide-part flag, allow-hidden-spawn flag, no-physics flag and simple-physics flag.

`EProcessStimulus` (`0xE3C86F4C`) is 16 bytes: event ID, target system hash,
stimulus hash and queue-mode hash. This connects damage zones to cargo systems.
`ESpawnExplosion` (`0x31BBBAE2`) supplies position-node hash, speed, radius,
start radius, affect-this, damage, include-players, exclude-vehicles and
exclude-smackables. There can be trailing alignment padding.

The `TrailerLog` engine has seven systems and three actions. Damage zones route
`IMPACT_1` or `COLLISION_COP` through `TriggerLoad` to `Cargo`. Its `DropLoad`
action starts a local explosion and emits nine fragment requests: four log
attachments, three posts and two attachment names absent from this model.
Missing parts must be ignored without hiding or inventing a mesh. The seven
present parts inherit 0.3 times the supplied impact velocity and hide their
parent attachment. These are measured values, read from the install at runtime.

`TrailerB` has seven systems and five actions. It drops different car attachments
on successive triggers. It must not be reduced to a single hardcoded log action.

## Scenery and breaker events

`EProcessAreaStimulus` (`0x762442F3`) stores system, stimulus, queue-mode and
radius at 4/8/12/16; the dynamic centre comes from the emitting model. Supports
send this message to distinct roof models. `EBreakerStopCops` (`0xDDA7829B`)
has radius and duration floats at 4/8. `EPursuitBreaker` (`0x47C1D36D`) has its
begin flag at 4. `EHidePart` (`0xEFFECDD6`) stores its part hash at 4.
EHideObject (`0x0713D2FA`), EKillObject (`0xBD2F6FF1`) and EWakeObject
(`0x8E60A642`) use dynamic model context. ETriggerMomentNIS (`0x7DF7F613`)
is a cinematic event and must not be decoded as a part-visibility command.
See [breaker behavior](../specs/pursuit-breakers.md) for the host semantics.
