# Per-side damaged brake surfaces

Sources: the PC light-state texture consumer and installed model texture
placeholders recorded in [player car lights](player-car-lights.md) and
[named lamp damage](vehicle-lamp-damage.md). This extends the surface adapter;
it adds no new crash damage threshold or lamp-break producer.

The three original brake bits (8,16,32) independently select OFF substitutions
for BRAKELIGHT_LEFT/RIGHT/CENTRE and their BRAKELIGHT_GLASS equivalents. A held
brake lights only intact sides. A missing authored OFF replacement retains the
available texture. Head diffuse/glass substitutions retain the independent
native head admission; a broken head flare does not turn off all head textures.

Cache surface appearances by both the actor's window damage and the three brake
bits. Share identical keys within a model rig, keep actors with different keys
independent, and retain the existing four head/brake input states. Upload only
solids whose actual texture replacements differ, deduplicating identical input
variants. Do not pre-upload the entire cross-product for all model parts.
Release variants when no live actor requests them and release all on rig teardown
or reshading. Repeated frames and switching existing input states must not add
GPU meshes. Player, traffic/police and separately posed trailer adapters supply
their own mask. The player rebuilds a cache cleared by a material change.

Tests must distinguish left/right/centre diffuse and glass selection with actual
installed textures, preserve head substitutions, isolate two actors sharing a
rig, combine window/brake damage, cover repair and cache resource retirement,
and verify the selected material output through native headless rendering.
Supplied named masks prove this consumer without claiming an unobserved general
crash-to-lamp producer. Native alpha/material optical agreement still needs a
matched original-game comparison.
