# Nitrous exhaust and taillight trails

Research checkpoint: 2026-10-17, written before implementation. This supplements
[exhaust-flames](exhaust-flames.md) and [player-car-lights](player-car-lights.md).
It specifies stock-PC effect routing separately from the optional restored
console wind contrails in [vehicle-visual-effects](vehicle-visual-effects.md).

Sources read: the PC executable identified in
[provenance](../provenance/nitrous-light-trails.md); the CC0 decompilation
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), revision
`80993f35224c61d2f1b135beba76c6f8234c7940`, particularly
`World/VehicleRenderConn.cpp`, `CarRender.cpp`, `CarRenderConn.cpp`; and the MIT
[Widescreen Fix NOS correction](https://github.com/ThirteenAG/WidescreenFixesPack/blob/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162/source/NFSMostWanted.WidescreenFix/NOSTrailFix.ixx).
No source implementation or game assets are copied. **[PC static]** below means
decompilation of the identified PC executable, not a runtime observation.

## Three independent effects

1. Nitrous exhaust uses the installed `ecar.NOSEffect` particle group while
   nitrous actually burns. It also serves the separate gear-change flame.
   **[decomp; existing exhaust data research]**
2. Taillight trails repeat the ordinary installed brake-lamp flares at sampled
   historical car positions while nitrous burns. They are present in the PC
   executable. **[PC static]**
3. Pale wind streaks use `fuelcell_effect/contrail`, with separate high-speed
   and platform rules. They are not the taillight effect, and must not be
   enabled to supply it. **[decomp; community restoration evidence]**

The nitrous button, bottle amount, an available upgrade or the exhaust shift
timer alone is insufficient to activate either nitrous visual. Consume the
physics' actual engaged/burning output, corresponding to `IsNOSEngaged`
through the draw-car packet's NOS flag. An empty bottle must show neither
continuous flame nor taillight trail even while the button remains pressed.
**[decomp; host integration requirement]**

## Exhaust routing

Use every supported installed exhaust marker and the already loaded group,
textures, emitter parameters and simulation from the exhaust specification.
During actual NOS engagement the selected group is `ecar.NOSEffect`, enabled
continuously at intensity 1. This branch precedes miss-shift, shifting and
Stop; it does not require an engine upgrade or the shift speed threshold.
When NOS ends, apply the ordinary shift/backfire decision for that step;
otherwise stop births and allow particles already alive to expire.
**[decomp]**

The previously verified default group `fxcar_nos` contains `emcar_nos_fire`
and `emcar_nos_glow`. The original textures are `FX_FIRE02_ADDITIVE` and
`FX_SMK06_BLEND`; resolve them through install data, with no procedural
replacement or hardcoded color for this route. Their short lives, local
emission frame, birth positions, motion-live/inheritance rules, drag and
curves remain defined by the existing exhaust specification.
**[existing verified-data and PC-static research]**

The remake's additive alpha gain of 4 is a previously documented host choice,
not an authored OG emitter value. Reusing it is compatible with the existing
effect renderer but must remain disclosed as an optical approximation. The
same applies to lift-off sputter flames: their sound coupling is a remake
addition and does not establish native nitrous behavior. Missing markers or
missing installed profiles produce no invented flame. **[host design]**

### Scope of the older emitter-frame investigation

The PC's actual pipe update routes through the persistent car `Effect::Update`
method. That method combines the effect's stored local matrix with the supplied
car render matrix, assigns it to the group, sets intensity/inherit velocity,
enables the group, clears one-shot mode and updates it. The reference car loader
constructs pipe effects from each exhaust marker matrix (identity plus authored
position when no marker exists). This supports the current marker-times-car
frame for NOS; it supplies no shortest-arc rotation from a pipe position.
**[PC static; decomp loader cross-check]**

The older exhaust specification also records an unresolved investigation of a
registered `World_OneShotEffect` handler and its descriptor-derived frame.
That separate path must not be treated as proof that the continuous NOS group
uses that handler or needs its reconstructed rotation. The particle draw pass
supplies an identity world frame to ordinary particle flares, independently of
the car-flare history path. Current world-space births are consistent with
these direct callers; exact optical agreement still requires a controlled
original capture. **[PC static; limited to these caller chains]**

## PC taillight history and gate

The PC car-flare dispatch first draws normal lamps. It then retains three car
render matrices and three translation vectors in a ring. History advances
for player views 1 and 2 during non-reflection draws, independently of NOS.
The extra trail is drawn only in a non-reflection pass, with speed magnitude
strictly above 0.2 m/s and the car's NOS state nonzero. A hidden/unrenderable
car or a missing car render object is skipped. The usual camera mover gate
also hides the anchored player's car in a POV that suppresses it.
**[PC static]**

These are lamp history samples, not exhaust particles and not points placed
along the current forward vector. Store full body orientation at each sample;
turning must leave the marker placements following that history.

For a current ring index `r`, visit three directed edges in this order:

| Edge | Start slot | End slot |
|---|---|---|
| 1 | `(r + 1) % 3` | `(r + 2) % 3` |
| 2 | `r` | `(r + 1) % 3` |
| 3 | `(r + 2) % 3` | `r` |

For each edge, emit eight repeated flare sets, with translation
`start_position + (end_position - start_position) * (j / 8)` for `j=1..8`.
Retain the start slot's orientation; do not interpolate the rotation. The
second edge closes the ring from newest to oldest: the native routine really
does include that overlapping segment. Eight is the initialized PC flare
division count. The resulting maximum is 24 extra flare sets per eligible
car, in addition to the normal current-position lamps. **[PC static]**

This is a sprite smear. A connected ribbon, stretched procedural rectangle,
new red texture, lifetime gradient or brighter replacement flare is not the
observed PC algorithm. There is no trail-particle lifetime: the additional
sets stop drawing as soon as the NOS/speed gate fails. **[PC static]**

## Lamp selection, state and draw

Each extra set calls the ordinary car flare path with brake-only selection.
It accepts flare type 1. These are installed left/right/centre brake markers;
headlights and reverse lamps are excluded. Traffic brake profiles are
converted to type 2 and do not pass this selection. Forced light state 8
suppresses headlamps rather than forcing braking. Consequently outer tails
retain ordinary 0.5 intensity, rising to 1 when braking, and the centre lamp
stays dark until braking. Broken/disabled light flags still apply in the
original; missing remake damage integration must remain a stated gap.
**[PC static; decomp marker names]**

Use the same installed racer brake profile and texture as the ordinary lens
flare. Apply the historical transform to marker position and direction, then
use the current camera for facing, profile power, depth/focal scaling,
authored size endpoints, bias and distance fade. Preserve additive
source-alpha/one blending. Repeated copies carry ordinary brightness; the
overlap creates the smear. Do not divide by the sample count or invent a
tail-specific opacity curve. Render with the existing world flare depth
policy before the HUD. See the player-car-light specification for the
appearance arithmetic and its unresolved occlusion/rotation details.
**[PC static; existing flare renderer research]**

## Host timing and lifecycle contract

The native history advances per eligible rendered view, so its spatial length
depends on frame rate. The Widescreen Fix corrects the position delta using
`(target_rate / 60) * custom_length`, where custom length defaults to 1;
its optional matrix-position correction is separately disabled by default.
This is community normalization, not stock-PC arithmetic.
**[PC static; community source]**

For the remake, an explicit fixed-60-Hz history is a host choice consistent
with its fixed simulation step and the community 60-Hz reference. Record one
sample per simulation step, rather than once per renderer invocation. This
avoids variable trails across 30/60/120-Hz output, duplicate history updates
from auxiliary passes, and invented time accumulation while paused. At
366 km/h, two 60-Hz intervals span approximately 3.39 m of straight travel;
the rotated lamp offsets also affect visible extent. **[host design]**

Rebase sampled translations by displayed-car translation minus newest physics
translation, retaining historical orientations. This keeps the smear anchored
to the interpolated car rather than leading it by up to one physics step.
This presentation-phase adjustment is a host choice. **[host design]**

Seed all three slots with the current pose on load/reset/car replacement.
Reset history after teleport/respawn or when leaving/entering a car-visible
camera so a stale location cannot bridge the scene. When no camera can draw
the player car, suppress trail drawing. Pausing freezes samples. NOS release
removes the extra sets immediately, while normal lamps remain. The exhaust
setting may control its existing particles; it must not silently control
taillight history. Keep optional wind-trail settings independent.
**[host design]**

The upstream blue nitrous tint (red/blue channel swap) is retained. It is a
host approximation alongside the additive gain, not an original-game colour
measurement. **[host design]**

## Coverage required before claiming parity

Verify actual engagement with stock/upgraded cars; exhaustion with a held
button; release; ordinary shifts; braking/centre lamps during NOS; stationary
or very slow NOS; turns; camera switches; pause; reset/respawn; car replacement;
missing markers/profiles; and reflection exclusion. Compare the same installed
car, camera and motion at 30/60/120-Hz output. Tests of counts and positions
can establish routing and timing, not an optical match. **[host validation]**

This checkpoint establishes the PC dispatch, sample geometry and gates from
static evidence. It does not establish a side-by-side visual match, original
occlusion query behavior, all-car marker correctness, remote/AI support, exact
surface bloom, original pipe-frame behavior in every car, or pixel parity of
the existing additive flame gain. Console wind trails, radial motion blur,
camera FOV and nitrous audio have separate specs and are not supplied by a
taillight smear. The full requested nitro presentation needs these limits
reported explicitly rather than claiming complete original-game parity.
