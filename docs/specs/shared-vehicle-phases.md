# Shared vehicle collision and response phases

Sources: original PC 1.3 executable (proprietary, behavior inspection and private
capture only); dbalatoni13/nfsmw rigid-body reference (CC0, decompiled, read for
behavior only). No source code is ported. See the provenance record below.

One shared simulation tick uses these ordered phases:

1. Capture each car's input, prior-pose terrain query and gameplay preparation.
2. Integrate all participating car and trailer bodies from their previous forces.
3. Resolve the existing deterministic car-pair sequence.
4. Finish every body: movable props, walls, body ground response, drag and sleep checks.
5. Resolve trailer articulations from the completed body state.
6. Run controls, engine, steering, suspension and chassis tail corrections.
7. Publish damage, sounds, gauges and interpolation poses once.

Use the same positive finite simulation delta in every phase. Invalid deltas
leave state unchanged. Preserve the default 60 Hz clock, optional comparison
cadence, network cadence and Speedbreaker scaling. Standalone simulation and
prediction use the same phases without a shared pair pass.

The body phase retains staging restraint after integration. Frozen bodies do
not respond and clear their pending force and torque at body begin. Sleeping bodies can wake from input or a collision before vehicle
response; do not cache an inactive result across the pair pass. Terrain-cache
queries retain their captured prior-pose origin. Engine and suspension read
post-contact body velocities rather than the integrated pre-contact values.

AI decisions, reaction parameters and RNG order are not retuned. Trailers
integrate before pairs and retain deterministic coupling and release behavior.
The prop-pair pass follows all body begins but remains separate from vehicle-prop
admission. Unified mixed-body pair traversal, trailer-release callback priority
and all original actor task priorities are not established by this contract.
See [prop and articulation phases](prop-articulation-phases.md).
For the Reference tire profile, an intact sleeping car still receives engine
and suspension response. Even a zero aerodynamic force wakes its body. Sleep
sets the state without clearing current vectors; the next sleeping body begin
clears them unless a force or contact wakes it first. The Regularized profile
retains its existing pedal-based wake policy.

A caller-tagged retired sleeper still receives engine response, while suspension
and chassis response pause. The retirement tag currently represents destroyed
non-player cars, not every possible destroyed player state. Reference body-end
sleep uses the prior support timer and object-contact veto. Its later support
callback advances that timer before suspension. World-only contact does not
substitute for the object-contact veto. Frozen actors remain excluded by the
host's inactive-mode policy; force methods independently ignore Frozen bodies.

See [provenance](../provenance/shared-vehicle-phases.md). Arithmetic boundary
replay and phase-order tests do not prove a complete original-game trajectory.
