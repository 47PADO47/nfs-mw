# World material routing

- **Spec:** [Original PC world material routing](../specs/world-material-routing.md).
- **Sources read for the spec:** retail PC 1.3 `speed.exe` shader descriptor,
  constructor/resource loader and embedded WORLD, TREE, WORLDNOFOG and
  WORLDPRELIT effects (proprietary; private disassembly, no code copied).
  [noclip.website MW renderer](https://github.com/magcius/noclip.website/tree/master/src/NeedForSpeedMostWanted)
  (MIT; read-only corroboration of family/vertex-format distinctions).
- **Date:** 2026-10-15. Specification precedes implementation.
- **Checked against the game by:** executable SHA-256 identity, bounded descriptor
  table and constructor/resource-loader inspection, D3DX shader disassembly,
  byte-exact comparison of all 31 embedded effects in stock and preferred modded
  executables. These are static observations, not a captured gameplay draw.
- **Known differences:** original projected shadow maps, reflective ID 1 and
  glossy-window ID 5 materials, runtime sampling/culling state and gameplay
  matching remain incomplete. Normal family 3 and native sky composition have
  separate specifications and independent original shader fixtures. The custom
  lighting/shadow enhancements remain independent.

The stock executable SHA-256 is
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
the preferred modded executable is
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
Their embedded shader resources match even though the complete executables differ.
The original shader descriptor table and constructor establish indices rather
than relying on an assumed order of resource names.
