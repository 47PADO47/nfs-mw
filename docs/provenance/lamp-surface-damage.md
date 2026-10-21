# Brake surface damage provenance

Spec: [lamp surface damage](../specs/lamp-surface-damage.md).

The prior bounded PC inspection of UpdateLightStateTextures at 0x737E30
established independent brake L/R/centre broken-bit checks and unconditional
head replacement assignments before the broken mask is fetched. It is recorded
in player-car-lights.md and the private original-game evidence. Model loaders
retain each installed side placeholder and its authored ON/OFF textures.
The CC0 VehicleFX reference names the same bit identities; no decompiled code
or installed textures are distributed.

The remake previously chose one brake Boolean for all three sides. The new
consumer retains per-side OFF selection and shares lazily created affected-solid
variants by live window/brake state, rather than duplicating every mesh for all
possible damage masks. This is host resource organization, not a claim that the
original used this cache. General head/tail crash triggers remain unestablished;
the named event and installed cop-bar producer are documented separately.
