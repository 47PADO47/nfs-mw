# blackbox-aems

AEMS of EA Black Box games (Need for Speed: Most Wanted `SOUND/**/*.abk`): a reader for the module banks and an
interpreter for the event-sound graphs inside them, the logic that decides which samples play, how loud and how
high.

An `.abk` file is a box of samples and a set of *modules*. A module is the logic of one sound class (the engine,
the shift sweeteners, the sputters, the turbo): a dataflow graph of small nodes (counters, random draws, tables,
envelopes, arithmetic, *players* that turn a sample group and a play control into a voice) with the compiled code
that runs the nodes in order. The game sets the class data (the parameters of the sound object), runs the module
once per tick, and the voices and objects the graph asks for are the host's.

- **Pure.** No files, no audio output, no game knowledge: bytes in, calls out. The same bank, parameters and seed
  give the same calls. Which class is which and how a voice plays a bank sound is the caller's.
- **Parse then instantiate.** `ModuleBank::parse(&[u8])` validates every offset and count. `Instance::new(&bank,
  module, seed)` copies the module's image and decodes its code once (a small, fixed x86 subset, interpreted).
- **Per tick.** `set_class_data(&[i32])`, then `update(period_ms, &mut host)`. The `Host` trait receives
  `play`, `stop`, `pause`, `resume` and `update` for voices (with the inputs: pitch, volume, ...) and `class_call`
  for the objects a class controller creates. `player_view` and `player_inputs` read the state back.
- **Never panics on bad data.** A bad offset, an unknown instruction or an unknown node function is an `Error`.
- **Written from the specs** in `docs/specs/aems.md` and `docs/formats/aems.md`, not from the decompilation
  (`docs/provenance/aems.md`).
- `minimal_bank()` builds a tiny bank, so tests and examples do not need a game install.

```rust
use blackbox_aems::{Instance, ModuleBank, NullHost};

let bank = ModuleBank::parse(&abk_bytes)?;
let module = bank.module_of("CAR").expect("an engine module");
let mut instance = Instance::new(&bank, module, 1)?;
instance.set_class_data(&params);
instance.update(1000.0 / 60.0, &mut host)?;
```

## Tests

`cargo test -p blackbox-aems`: every node on synthetic state (tables, envelopes, ramps, oscillators, delay lines,
counters, shuffles, arithmetic, players, class controllers, destroy), the code decoder, parsing and truncation,
and a whole instance on the in-memory bank. With `NFSMW_GAME_DIR` set and `--include-ignored`: all 386 shipped
modules run without error on several parameter sets, and the M3's engine module layers its samples over the RPM
range.

License: MIT OR Apache-2.0.
