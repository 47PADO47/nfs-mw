# Sequenced prop particles

The behavior contract is `docs/specs/prop-object-effects.md`. Read-only inspection
on 2026-10-20 of the matching PC 1.3 executable established the object-effect
constructor and dispatch fields, keyed versus world effect selection, marker
lookup and stop handlers. Installed world sequence strings and inherited effects,
emitter groups and particle textures establish the runtime data links.

The implementation uses the existing clean Rust particle simulator and texture
reader. No original code, assets, extracted tables or decompilation are included.
Missing names, markers and art skip an effect without aborting destruction.

Verified structure does not establish visual parity. The earlier zero-cycle
single-burst adapter was replaced after isolated original PC instruction execution
confirmed continuous emission in that case. See `specs/particle-emission-control.md`.
Wrapper attributes and atlas animation are covered by `prop-particle-attributes.md`.
Full audio/cinematic integration, effect ordering and matched original-game
driving captures remain outstanding.
Both prop solvers consume the same sequenced visual events. Synthetic checks
cover ownership, stops, budgets and finite bursts; installed checks trigger real
support and pump sequences and resolve their particle textures into draw batches.
