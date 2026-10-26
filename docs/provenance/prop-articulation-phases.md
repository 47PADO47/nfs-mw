# Prop and articulation phase provenance

Date: 2026-10-26. Spec: [prop and articulation phases](../specs/prop-articulation-phases.md).
Sources: original PC scheduler and body-end routines (proprietary, static behavior
inspection); dbalatoni13/nfsmw `RigidBody.cpp` reference (CC0, decompiled,
behavior-only corroboration). No source code ported.

The retained [shared-phase provenance](shared-vehicle-phases.md) establishes all
body finishes before articulation and later engine/suspension work. Generic
body-end inspection corroborates world/ground response before drag and sleep.
Current Rust producer/caller tracing identifies completed-contact consumers
that still run inside the earlier pair callback. Tests exercise those boundaries
without shipping original binaries, recordings or installed assets.

The intrusive recording does not establish normal-FPS trajectory equivalence.
Unified mixed-body pair order and exact trailer release-task priority remain
unverified. Existing hitch tuning and AI driver decisions are preserved.
