# blackbox-mixmap

The dynamic mixer of EA Black Box games (Need for Speed: Most Wanted `SOUND/MIXMAPS/*.mxb`): a parser for the
maps and a deterministic evaluator that turns the values a game publishes into the volumes, pitches and filters
of its sound objects.

A map is a graph the sound system evaluates once per frame. *Controls* turn published values (the car's speed, a
nitrous flag) into levels through curves; *events* are envelopes (attack-release, attack-sustain-release,
attack-trigger-release) that a trigger starts; *3D controls* turn a distance and an azimuth into a rolloff;
*sub-mix channels* add levels; each *master channel* adds its inputs to a base level and writes the result to
output slots (volume, pitch in cents, filter, azimuth) of one sound object. A map has up to 13 states, one per
kind of sound object, instanced once per object of that kind.

- **Pure.** No files, no audio output, no game knowledge: bytes in, numbers out. The same map and the same calls
  always give the same slots. Which object is which, what the inputs mean and what a slot means to an object is
  the game's table.
- **Parse then instantiate.** `MixMap::parse(&[u8])` validates every offset and count and returns plain structs.
  `Mixer::new(Arc<MixMap>, &[instances per state])` resolves every id to the element or input it reads.
- **Per frame.** `set_input` for what changed, `attach` for the objects that exist, `process(dt)`, then read
  `volume`, `pitch`, `filter` or `azimuth` of an object's slot.
- **Written from the specs** in `docs/specs/dynamic-mixer.md` and `docs/formats/mixmap.md`, not from the
  decompilation (`docs/provenance/dynamic-mixer.md`).
- `MapBuilder` writes maps, so tests and examples do not need a game install.

```rust
use std::sync::Arc;
use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};

let map = Arc::new(MixMap::parse(&mxb_bytes)?);
let mut mixer = Mixer::new(map, &[1, 0, 1]);        // one main state, one player-car state
let engine = ObjectRef { state: 2, instance: 0, object: 2 };
mixer.attach(engine, true);
mixer.set_input(InputKey::controller(2, 0, 0, 0), 12_000); // controller 0, input 0
mixer.process(1.0 / 60.0);
let gain = mixer.volume(engine, 2);                  // Option<f32>
```

## Units

Levels are hundredths of a dB (-10000 = silence) inside the graph and Q15 (`0x7FFF` = unity) in volume and
filter slots; pitch slots hold cents. `q15_from_db`, `db_from_q15`, `curve`, `curve_db` and `pitch_ratio` are
public.

## Tests

`cargo test -p blackbox-mixmap`: the shapes and conversions (round trips, end points, monotonicity), the ids,
parsing (round trip of every element kind, truncation, bad offsets and counts, garbage input), evaluation on
synthetic maps (a control's cut and offset, scales, sub-mix clamping, master base level and conversion,
attached and unattached objects, filter and pitch slots, instances and cross-state expansion, determinism,
non-finite inputs), the three envelope kinds, and the 3D rolloff (distance, range, azimuth blending, camera
state). The four real maps are tested in `crates/nfsmw` (`audio::mixer`, needs `NFSMW_GAME_DIR`).

License: MIT OR Apache-2.0.
