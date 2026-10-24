# Physical player city pursuit acceptance

Date: 2026-10-24. Specification:
[physical target acceptance](../specs/ai-pursuit-physical-target-acceptance.md).

The fixture reuses the project's installed road/collision setup, ordinary
`CarSim` player stepping, native-style PID controller, installed BMW physics and
real heat-five fleet loading. No new native formula is introduced. A privately
supplied trace provides route geometry only; no capture coordinates, controls
or trajectories are embedded in public source.

The rolling speed, duration, progress, bend/grade coverage and engagement gates
are engineering criteria declared before execution. They do not derive from
an original cohort survival guarantee. Original native capture analysis shows
that early ordinary units stop appearing while later replacements close the gap.
The capture does not establish why those earlier observations end.
The existing recorded-target same-unit stress remains unchanged and failed.

Both city setups prepare the accepted navigator with the selected cop's width
and trail before spawn and handoff checks. This matches the corrected native
trail-centre clamp. Raw-lane and prepared-trail centres can differ; changing
the fixture input does not relax placement tolerance or chase scoring.

Implementation is complete, but a valid physical BMW run and police acceptance
remain separate pending results. The current pilot has not met its route gate.

The first physical run completed with mean speed 61.689 m/s, 2,127.6 m of route
progress, 2,089 supported ticks out of 2,100, substantial bends and 28.54 m of
height range. It was invalid because route deviation exceeded 8 m for 131
continuous ticks, with a maximum of 11.393 m. Its police scores cannot establish
acceptance. The principal invalid interval followed a bend and persisted onto
a flatter route section; chord error alone did not predict actual body error.

The revised test pilot bounds only steering-preview chord deviation by the
physical BMW's full width. It keeps speed preview, physical controls and every
acceptance gate unchanged. This is an independently written geometric fixture
correction, not a change to police behavior or a native steering-gain claim.
That revision also remained invalid: maximum route error 9.931 m, with 95
continuous ticks outside the 8 m tube. Police scores remain unaccepted.
The ordinary player's speed-sensitive input shaping differs from the AI
controller's steering-angle normalization. That fixture contract needs review
before further preview adjustments. No acceptance threshold was lowered.
