# Sequenced prop particle effects

Sources: installed world event packages and inherited effects/emitter attributes;
matching PC 1.3 object-effect constructor/handler, stop handlers and world one-shot
effect service inspected read-only on 2026-10-20. Existing particle/exhaust specs
describe emitter parameters. Original bytes and decompilation remain private.

EPlayObjectEffect serialized fields: effect string tag at 4, bounds-position CRC
at 8, intensity float at 12, instance ID at 16, tracking flag at 20 and
allow-hidden-spawn flag at 24. Resolve the name through the owning package's
strings, then the effects attribute class and linked emitter group. Missing
strings, effects or nonzero markers skip the effect, not the destruction sequence.
Zero means absent marker/instance ID. Intensity clamps to [0,1]; it is not time.
Some audio-named wrappers also contain particles; do not classify by names.

With no instance ID, fire a detached world one-shot. Positive emitter OnCycle
and StartDelay govern emission; Life governs surviving particles. With an ID,
create/update the model's keyed effect. Tracking follows the source marker/model;
untracked keyed effects keep their initial world frame. Stop-one carries ID at 4;
stop-all ends that model's keyed effects. Stop spawning and let live particles
expire. Source removal/reset ends keyed emission. A detached burst can outlive it.

The host queues events in sequence order on the fixed physics clock and resolves
textures on the renderer thread. Cache templates/textures and bound instances,
emitters and particle counts. Original and Enhanced modes share this visual path.
Hidden models skip starts unless explicitly allowed. Named immediate collision-bound
children supply the emission position and rotation. The marker's up direction
orients the particle frame; no marker uses the model translation and world up.
Render-space transforms and inherited velocity must agree.

Emitter phase and boundary behavior follow `particle-emission-control.md`.
Zero-OnCycle forced emitters stay on and use life-scaled batches, as confirmed
with isolated original PC instructions. Wrapper intensity, inherited velocity,
group activation ranges and quantized atlas frames follow
`prop-particle-attributes.md`. Exact original pixels, global resource competition,
keyed model-effect audio and matched live captures remain open. Standalone
sequenced one-shot audio now follows [scenery effect audio](scenery-effect-audio.md)
independently of the particle group.
Avoid duplicating the old petrol-station visual fallback when the authored event
successfully resolves a particle group; retain its explosion audio notification.
