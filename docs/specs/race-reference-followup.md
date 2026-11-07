# Custom-race reference follow-up

Behavioral findings from a complete modded PC sprint, recorded 4 November 2026.
Provenance and limits: [race-reference-followup](../provenance/race-reference-followup.md).
This supplements [ai-racers](ai-racers.md), [race-results](race-results.md) and
[race-finish-effects](race-finish-effects.md).

## Racer action performance

Race actions must use the analytical AI performance estimate described in
[ai-pursuit-acceleration-handoff](ai-pursuit-acceleration-handoff.md). This is the
same planning domain for an opponent's own curve and the player's matching
curve. A physical acceleration trial is a separate measurement, with launch,
wheel-slip and shift delays that do not belong in the native action estimate.
The physical vehicle still uses its ordinary engine, tires and controls.
Unsupported estimates retain the existing explicit measured fallback. **[exe]**

The integrated police adapter already selects this estimate. Race grid creation
must select it for both sides of the match as well. Keeping one physical curve
still limits requests through the minimum used by acceleration matching.
This repairs an integration mismatch; it does not establish complete racer AI,
performance-part selection, nitrous or catch-up parity.

## Off-path recovery

Restore the non-pursuit off-path check specified in
[ai-driver-speed-skill](ai-driver-speed-skill.md), section 1. At the four-tick
check, a car outside its trail corridor by more than 2 m can reacquire a closer
road only when that road belongs to neither its current path nor its cookie
trail. Initialize the new cursor at the car on a valid racing lane, then restore
its route search and look-ahead. Do not replace a valid route just because the
car's centre lies beyond a fixed distance from the route centre line.

## Shortcut barrier context

Shortcut searches must receive the active race barrier lines, as the base route
search does. Search on a private copy of the road flags, preserving the grid-road
exemption and segments already accepted by that base leg's fallback policy.
Search both halves through the shortcut marker with those restrictions. Keep the
existing length limit; reject a shortcut with no complete allowed route.

This corrects the remake's shortcut integration. The prior search ran after the
base route had cleared its temporary flags. On the recorded course, that produced
a shorter path into an active arrow wall while the base route went around it.
Do not change a racer's body collision mask to let it drive through that wall.
The temporary search flags must not change police navigation or free-roam flags.

## Sprint winner photo

The sprint photo's speed is the player's speed at the finish, not its average
over the race. **[verified runtime + video]** The captured player finishes at
about 99.54 m/s; the photo displays 358 km/h. The elapsed race time is about
108.47 s over a roughly 7.95 km course, which would give a different average.
Keep the already separate average-speed statistic and record unchanged.
Other race modes retain their existing policy until independently checked.

## Sprint opponent result times

When the sprint ends on the player's finish, opponents still racing receive a
projected result time. For a non-eliminated AI racer with positive completion
percentage `p`:

```text
projected_time = race_elapsed * 100 / p
```

The six observed rivals have about 86..91 percent completion when the player
finishes. Their final timer values match this projection; they do not all drive
over the finish line at that instant. **[verified runtime]**

Use the projection only in the finished sprint's result report. Keep an actual
finish time when available. Do not fabricate a time for a human who did not
finish, an eliminated racer, zero progress, invalid progress or a time-limit
loss with no human finish. Do not change checkpoints, physical position or
the engine's `finished_at` to generate a results cell. Time-behind uses the
reported time against the actual winner's time. Racer averages use that racer's
reported timer, consistent with the original statistics reader.

This does not infer remaining checkpoint/stage times or generalize the rule to
knockout, drag, speed-trap or circuit results without evidence.

## Intentional custom options

Larger grids and configurable police in custom races are intentional remake
features. Preserve them while repairing reference behavior. The observed modded
reference itself uses six opponents. A stock grid limit is not an acceptance
limit for an extended custom race.

Police enablement, starting heat, heat limits and population/tactics are separate
concepts. The existing UI remains an extension; a clearer heat-based control and
possible separate settings are future design work, not rules established by this
police-disabled recording.

## Validation boundaries

The reference covers options, intro, countdown, launch, racing and results, with
all seven racers linked to physical telemetry and no recorded race-probe drops
or read errors. It includes complex/simple simulation transitions. A short
audio gap limits sound-timing conclusions, not the retained race telemetry.
It does not establish every difficulty, police setting or race mode.

## Race presentation follow-up

Custom race `Heat Level: None` overrides the selected track's `CopsInRace` flag. Career races without an explicit option keep their authored flag. Pursuit events retain the police required by their rules.

Race intro engines use the installed `SOUND/NISREVDATA/<scene>.bin` curves. The file has sixteen channel sets, each a little-endian count followed by `(f32 time, i32 RPM, i32 torque)` points. Ignore trailing bytes after the sixteen sets. Reject truncated, non-finite or decreasing timelines. Interpolate RPM and torque in scene elapsed time; do not run vehicle physics to produce cutscene revs. Movies retain their own soundtrack.

Remove the fixed six-Hz player-only rocking loop. Body motion must follow authored car animation or scene events, rather than continue whenever the car is stationary. Race car channels bind to the race grid in racer order, independently of nearby civilian traffic. Cars beyond the authored channel count keep their grid poses.



The complete NIS stream continues after the intro camera ends, through the countdown and its GO tail. A skip or restart
selects the installed end section (section selector bit 2), rather than restarting the complete intro. Keep the request
until race teardown; playback remains a one-shot and is not repeated after natural completion.

Authored car events use the CARP emission's system hash (`vlt_hash("car1")` through `car8`, then `cop1` through `cop8`),
not the order in which events fire. Pitch/roll fields are dip distances and pulse lengths. Convert dip using half the
wheelbase/track width; the controller's normalized angle is `atan2(dip, half_span) / pi`. Its triangular envelope reaches
one half at mid-length and returns to zero at the end. Burnout adds its signed linear speed divided by tyre radius to
only the driven wheels. These behavioral contracts derive from the reference declarations; PC runtime amplitude and
frame-perfect visual matching remain unverified. No reference implementation is copied.

A flat scene-plane camera can intersect a sloping streamed road. The remake keeps the start camera at least its near-plane
distance above the first nearby ground face, probing only three metres above the eye to avoid selecting bridges overhead.
This is an explicit remake collision safeguard, not a measured original camera-clipping rule.

The second recording uses Hard, six opponents, Minimum traffic, catch-up Off and police disabled. The human finishes
second. Its forced rival finish callbacks occur later than the human finish: projecting all timers at human finish does
not reproduce those results. The projection boundary needs further comparison; do not treat the first winning capture's
formula agreement as general result-screen parity.


A route window can revisit a road segment. Retire a path prefix only when the navigator accepts the edge to its next
segment; turning around on the same segment retires nothing. Periodic racer route refreshes retain an already-consumed
suffix when it still belongs to the same window and goal. Never select the first occurrence again merely because a road
ID repeats. This corrects path traversal identity, without advancing gates, teleporting cars or using distance heuristics.

The PC start-audio selector table uses enum flags, not its row number: IntroNis01 through IntroNis10 use bits 0 through 9; GenericStart is bit 18;
Flythrough 12; D-Day 14; D-Day end 19; Toll 17; Garage enter/exit 13; OPM 15. Missing start takes stay silent instead of
playing a different scene's first take. In the installed table IntroNis04 selects full take 4 and end take 5, not takes 6
and 7. These are independently checked against PC table initialization. Stream end-section length includes authored tails;
it is not required to equal the three-second gameplay countdown. Engine curves already use the sound RPM scale, so they
bypass the vehicle RPM remap while a scene owns them.
