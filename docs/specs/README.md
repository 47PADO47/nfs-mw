# Behaviour specs

Plain-language specifications of game behaviour (physics, AI, pursuit, streaming, camera, …), written
before the Rust implementation. Required wherever the only source is decompiled code; see
[licensing.md § Spec-first](../licensing.md#spec-first).

Written so far: [scenery-visibility.md](scenery-visibility.md) (exclude flags),
[scenery-lod.md](scenery-lod.md) (level of detail) and [car-assembly.md](car-assembly.md) (stock parts,
wheels, brakes, paint, texture swaps, the car shader). Planned: vehicle physics (`vehicle-physics.md`:
chassis, suspension, tires, engine, transmission).

## Template

```markdown
# <topic>

- **Sources read:** <URL, license, files>
- **Data inputs:** <AttribSys classes and fields, chunk types>

## Behaviour
<state machines, formulas, update order, units; pseudocode where useful, never translated source>

## Constants
<only where a constant is hard-coded in the game rather than in AttribSys; say where it was observed>

## How to check it
<what to measure in the original game to confirm the implementation matches>
```
