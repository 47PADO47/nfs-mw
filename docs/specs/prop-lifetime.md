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

`ALLOW_OFF_WORLD=false` kills a simulation after one continuous second with an
invalid world position or with its entire bounding sphere below the queried
world height. A valid position resets this clock. The rewrite adapts this to a
downward world-face query; the original cached `WWorldPos` behavior is separate.

The placed scenery model's destructor clears its spawner's model reference and
its scenery override hide bit. Thus releasing that model restores the original
placement. This is not a `RESPAWN_TIME` countdown: that attribute is retained, but
no read of its key was found in the inspected PC executable. The rewrite restores
released placements and removes released fragment bodies from collision and draw
registration. Streaming still reconstructs placements when their tile returns.

All clocks and sinking movement advance on the shared 60 Hz world timeline.
Tests must cover visible/invisible transitions, exact expiry boundaries, resting
without dropout, sinking without ground reattachment, off-world exceptions,
parent release, placement restoration and identical traces across render rates.
Live original-game timing, static trigger recreation and occlusion remain open.
