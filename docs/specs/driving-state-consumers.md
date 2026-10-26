# Driving state consumers

Sources: current Rust producer/caller audit; existing PC collision and shared-phase
contracts (proprietary behavior inspection and CC0 reference corroboration;
no restricted code ported). This is state-flow correctness, not full OG parity.

A completed driving tick publishes one current pose, telemetry, gauge sample and
effect velocity. Current-state observers must consume that same tick. In
particular, a nitrous trail must use the completed tick's nitrous flag, including
both press and release, rather than a previously published display snapshot.

Keep interpolation between completed poses and gauge samples. Keep discrete
gear and nitrous states current. Preserve captured raw pedals where their
consumer contract requires driver intent; shaped pedals are a distinct input.
Do not replace all throttle fields with a single supposedly universal value.

Collision severity uses the contact's incoming velocity and impulse. Camera
acceleration uses the before/after tick velocities with the tick delta. Preserve
these historical inputs: replacing them with final velocity loses impact data.

Publish property collisions, damaged-property costs, pursuit player observation
and traffic state/audio frames after the current response and contact publication.
Keep actual pair solving and per-contact damage snapshots in their pair phase.
Consume each completed contact once, including the final tick of an event.

Prediction snapshots must retain terrain selection histories that influence
future wheel/ground response. Selected face and retained miss state are model
history, even when the owner calls them caches. Pure acceleration structures
may be discarded and rebuilt. Do not rewind externally owned damage/event side
effects as part of restoring physical motion.

This contract does not establish the original cadence of frame-level meter
updates, every audio pedal source, or exact effect presentation timing.

Speedbreaker button edges, eligibility and real-time depletion remain loaded-frame
operations, including frames without a physics tick. Resolve these before choosing
the frame's clock target and vehicle effects, so an exhausted meter cannot leave
slow entry deltas paired with inactive handling and permitted nitrous recharge.
Passive recharge runs after each completed player tick using its speedometer
speed and simulation delta; never apply a batch's future time to its initial speed.
Visual camera/look updates do not mutate meter activity. This is a coherent
remake policy, not proof of exact OG exhaustion cadence or equivalent activation
transitions under all render-frame groupings.
