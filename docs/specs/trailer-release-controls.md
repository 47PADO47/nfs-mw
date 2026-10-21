# Trailer release controls and wheel support

Evidence and boundaries: [provenance](../provenance/trailer-release-controls.md).

While hitched, the trailer receives its tractor's brake and handbrake inputs.
When the hitch releases, clear both inputs. A loose trailer rolls freely under
its own suspension, contact forces and drag; it does not receive an invented
full-brake command. Later tractor inputs must not brake a detached trailer.

The off-ground release check uses one elapsed timer for the coupled pair.
Advance it while either body has fewer than two supported wheels. Reset it
only when both bodies have at least two. Release strictly after two seconds.
Alternating which body lacks support does not restart the timer. The existing
upright and relative-up-vector release thresholds remain separate checks.

The remake evaluates these rules on its shared 60 Hz timeline. This correction
does not establish the original task cadence, complete traffic suspension or
the cause of every truck seen bouncing without an obvious impact. Installed
startup, curved-road and cargo checks remain necessary controls; released
trailer tests must compare the actual runtime step with an independently stepped
brake-free body, including continued braking by the former tractor.
