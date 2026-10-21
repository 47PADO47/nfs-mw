# Scenery and detached-part lifetime

Sources: the user's hashed PC 1.3 executable, isolated lifecycle routines, and
the CC0 reference `SimModel.cpp`, `DrawVehicle.cpp`, `SmackableRender.cpp`.
No original code or installation data is distributed.

Placed scenery starts a temporary simulation on contact. Its positive
`KILL_OFF_SCREEN` countdown is loaded when simulation starts, reset to the
attribute value while visible, and decremented while invisible. Expiry uses a
strict negative comparison. Zero/negative values disable this task. Visibility
comes from the model's render visibility; the rewrite uses its current bounds
against the player camera frustum on each fixed tick. Occlusion/LOD visibility
equivalence remains open.

A detached vehicle part instead accumulates invisible time, resets it to zero
while visible, and releases the model when time is strictly greater than the
positive allowance. Releasing a parent releases its children. Removing a child
does not restore the parent's hidden mesh or spawn another load.

For a sleeping, unattached scenery body without a new object contact, the
original waits 0.125 seconds before ending simulation. A positive `DROPOUT[0]`
starts a sinking phase only while the body is off-screen (a rewrite gate: a visible
prop that comes to rest keeps its pose, so a displaced prop never snaps back in view): disable modeling/triggering, move the body downward by
`dt * DROPOUT[1]` per update, then kill the simulation when its remaining time is
nonpositive. A body with no dropout may retain its final pose as static scenery;
its simulation offscreen countdown stops. Subsequent contact can reactivate it.
Detached parts retain their separate model offscreen task after resting.

Here "detached parts" means vehicle parts (`DrawVehicle::Part`), not every
child model. **[decomp]** Scenery fragments spawned by a breaker use the same
placeable-scenery model as other dynamic scenery. Their simulation-owned
offscreen countdown stops when they settle without dropout. They remain at
their final pose until reactivated, explicitly killed, or released with their
parent. They must not inherit the vehicle-part invisible-time accumulator.
Fragments requested with `NoPhysics` never begin a simulation and therefore
never activate its offscreen, rest, dropout or off-world cleanup tasks. Their
event sequencer and parent ownership still work normally.

The visible-dropout adapter keeps a resting simulation's cleanup task pending
while it is visible, so looking away later can start the sink. It must not
permanently disable that task merely because the object rested in view.

**[decomp]** Hiding a scenery model (`EHideObject` or a fragment's `HideParent`)
ends its drawing and detaches its simulation. The smackable's detach callback
kills that physical simulation. The logical model, its event state, hidden
parts, consumed-breaker flag and child ownership remain; this is not model
release or placement restoration. Its hidden parent must neither collide nor
run an offscreen countdown that later rebuilds the intact structure. Sequences
and physical children continue independently. An explicit `EKillObject` releases
the model and children and may restore its original placement as before.

`ALLOW_OFF_WORLD=false` kills a simulation after one continuous second with an
invalid world position or with its entire bounding sphere below the queried
world height. A valid position resets this clock. The rewrite adapts this to a
downward world-face query; the original cached `WWorldPos` behavior is separate.

The placed scenery model's destructor clears its spawner's model reference and
its scenery override hide bit. Thus releasing that model restores the original
placement. This is not a `RESPAWN_TIME` countdown: that attribute is retained, but
no read of its key was found in the inspected PC executable. The rewrite restores
released placements and removes released fragment bodies from collision and draw
registration.

## Streaming and destroyed structures

**[decomp]** Streaming has a separate, section-scoped restoration policy. Before
unloading a section, the PC game records the time and each spawner's scenery
override hide bit. Unloading releases models and children, but does not mean the
placements become intact. Reloading within 180 simulated seconds preserves the
hidden spawners. A preserved hidden spawner starts without an active sequence,
simulation or detached children: it must not repeat a collapse, blast or cop zone.
Only when `unloaded_at + 180 < now` can a reload forget those bits; equality still
preserves them. The race status's `ActivelyRacing` latch bypasses that expiry
check; an ordinary free-roam pursuit does not. Time continues advancing;
the gate does not pause the elapsed-time clock.

This cache applies to ordinary activated scenery as well as pursuit breakers;
there is no separate six-second breaker respawn timer in these routines. Models
that remain resident still use their authored lifetime/sequence behavior. An
expired or pristine reloaded section restores its breaker map icons; global
`World_RestoreProps` restores all placements and icons and clears the cache.
The rewrite retains only hidden IDs and consumed-breaker metadata across tile
unload, never meshes, bodies, textures or child ownership. The original's finite
section-cache eviction policy and 2048-bit section limit remain separate limits.

All clocks and sinking movement advance on the shared 60 Hz world timeline.
Tests must cover visible/invisible transitions, exact expiry boundaries, resting
without dropout, sinking without ground reattachment, off-world exceptions,
parent release, placement restoration and identical traces across render rates.
Live original-game timing, static trigger recreation and occlusion remain open.
