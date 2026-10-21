# Source-owned collision particles

**[PC verified]** Collision VFX consume each physical owner's callback separately.
Audio selection and the damage sequencer do not decide whether these callbacks
exist. The owner supplies its `pvehicle` or `smackable` attributes; the other
surface supplies material selection. Other handles supply the actee identity,
not an alternative effect profile. See [provenance](../provenance/source-contact-effects.md).

## Selection

Read the 32-byte `EffectLinkageRecord` arrays for OnHitWorld, OnHitObject,
OnHitGround, OnScrapeWorld, OnScrapeObject, OnScrapeGround, OnBottomOut,
OnBottomScrape and OnDetached. Seed the default surface record, then use the
first matching other-surface ancestor. An explicit null/unsupported effect
blocks fallback to another material's effect.

Hits require squared closing-vector length above 1 and a positive clamped
`(impact-min)/(max-min)` ramp. Ordinary hit impact is the owner's notification
normal impulse divided by its stored mass. Smackable detachment uses closing
magnitude instead; an accepted detached hit returns before ordinary hit/scrape.
It remains accepted if its particles are absent or visually culled.
For a detached second pair owner the original passes the pair's unflipped normal;
the base object callback still uses its owner-facing flipped normal.

Scrapes use the callback's tangential vector, require speed above 1,
speed >= minimum and maximum > minimum, and clamp the ramp to 0.1–1.
The last accepted scrape updates the owner's persistent effect. Its original
timeout is 0.2 seconds; invalid selections pause it, and an absent callback
lets the timeout expire. Hit and scrape are independent in ordinary callbacks.

With more than two wheels grounded and up dot normal >0.2, vehicle ground
callbacks select OnBottomOut/OnBottomScrape at body origin minus half height
along up minus 0.75 half length along forward. Bottom-out impact is incoming
normal speed, rather than impulse delta-v.

## Emission

Apply the selected wrapper's inherited EmitterQuadratic once to the physical
ramp, clamped to 0–1. One-shots include the VisualCullDist boundary; persistent
effects require a strictly nearer position. Use authored PC emitter groups,
textures, atlas frames and native emitter clocks. One-shots force one-shot
control; scrapes retain continuous clocks. The owner velocity is multiplied by
wrapper InheritVelocity separately from each emitter's motion inheritance.
The original one-shot frame rotates physics up onto the contact normal before
swizzling both matrix bases to render axes (`S * R * inverse(S)`); persistent
frames follow their original normal frame.

Callbacks are queued independently of sound and camera-impact drains for the
player, AI vehicles, native scenery, and native fragments. The shared effect
layer delivers particles to either renderer. The previous player-only stock
world-spark emitter is disabled when this source-owned path is active; optional
experimental streaks remain a separate appearance setting.

## Limits

Existing PC profile validation rejects unsupported particle controls instead of
substituting another effect. Particle/source budgets are bounded. Callback
positions and relative velocities follow the implemented collision pipeline;
unimplemented original collision geometry cannot be reconstructed by this layer.
Matched source decisions and installed fixtures establish selection and renderer
delivery, not complete image equivalence for every original effect.

Tracked object-scrape service packets additionally transform their stored position
and direction through the owner frame in the original. That owner-relative
tracking transform is not reconstructed here; object-scrape particles currently
retain the callback world frame. World/ground scrapes and untracked hits do not
need that transform. Source profiles use a bounded least-recently-used cache;
missing textures log once per hash and are negatively cached until scene release.

The collision-particle setting clears sources and live particles and drops callbacks
while disabled. Experimental streaks remain optional supplements; they do not
replace an owner's authored native group selection.

While a scrape waits for another callback (at most 0.2 seconds), this host retains
the last callback frame, owner velocity and visual-cull decision. The original
refreshes current owner velocity and camera distance on every service update;
that intervening refresh remains a parity gap. A zero wrapper-mapped intensity
holds existing native emitter clocks while particles drain; one-shots at zero
mapped intensity do not create clocks.
