# Contributing

## Rules

1. **Never commit game data.** That covers files from the install, extracted textures, models, audio or
   video, and dumps of them. The game is read at runtime from the user's own copy.
2. **Never commit decompiled code**, whether from the dbalatoni13 decomp, Ghidra or IDA output, or
   anyone else's "decomp" folder. Behaviour that is only known from decompiled code (physics, AI, …) is
   written up as a spec in `docs/specs/` first and implemented from the spec. See
   [docs/licensing.md](docs/licensing.md#spec-first).
3. **Only port code from permissively licensed projects** (MIT, BSD, ISC, Apache-2.0, Zlib, CC0). Keep the
   original notice in the file and add the project to [NOTICE](NOTICE). GPL, LGPL, AGPL and unlicensed
   projects are references only.
4. **Credit what you learn from**: in NOTICE, and next to the claim in the docs.

## Guard rails

- **`.gitignore` is a whitelist.** New kinds of files are ignored until you add a narrow pattern for them.
- **Leak check:** `cargo xtask leak-check` refuses game-data extensions and signatures, binaries, files
  over 512 KiB, `.env`, decompiler names (`FUN_00401850`, `DAT_…`) and hex literals in `speed.exe`'s
  address range in Rust code. A literal that isn't an address can be marked with `// leak-check: allow`.
- **Hook:** run `cargo xtask install-hooks` once per clone, so the leak check runs before each commit.
- **CI** runs the leak check, then fmt, clippy (`-D warnings`), tests on Windows and Linux, and
  `cargo deny check licenses`.

## Workflow

```sh
cp .env.example .env          # then edit NFSMW_GAME_DIR
cargo xtask install-hooks
cargo test --workspace        # synthetic-data tests
cargo test -p nfsmw -- --ignored   # tests against your install
cargo run --release -p nfsmw -- view-car BMWM3GTR
```

- Formats are documented in [docs/formats](docs/formats) with evidence tags (**[verified]**,
  **[decomp]**, **[community]**, **[unconfirmed]**). Update the doc when a reader learns something new.
- Format crates take `&[u8]`; only `nfsmw-install` opens game files ([docs/architecture.md](docs/architecture.md)).
- Tests that need the game are `#[ignore]` and read `NFSMW_GAME_DIR`; everything else uses synthetic bytes.
