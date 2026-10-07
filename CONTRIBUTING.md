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

## Code organisation

- **Engine-generic code goes in [`libs/`](libs)**: codecs, format readers, install discovery, rendering;
  anything that also applies to other EA Black Box games (Underground 2, Carbon, …). No NFS:MW-specific
  names, paths or defaults there. Put game differences in per-version layout tables
  (`layout/v5.rs`, `layout/most_wanted.rs`) chosen from the data or passed in by the caller. Each lib keeps a
  README. The goal is that `libs/` can move to a shared utilities repo unchanged.
- **NFS:MW-specific code goes in [`crates/`](crates)**, which depends on `libs/`, never the reverse.
- **No file over 500 lines** (source or docs). Split by domain into folders and modules, one struct or
  concern per file; tests can live in a `tests/` module folder. `cargo xtask size-check` enforces it.

## Guard rails

- **`.gitignore` is a whitelist.** New kinds of files are ignored until you add a narrow pattern for them.
- **Leak check:** `cargo xtask leak-check` refuses game-data extensions and signatures, binaries, files
  over 512 KiB, `.env`, decompiler names (`FUN_00401850`, `DAT_…`) and hex literals in `speed.exe`'s
  address range in Rust code. A literal that isn't an address can be marked with `// leak-check: allow`.
- **Hook:** run `cargo xtask install-hooks` once per clone, so `cargo xtask check` (leak + size checks) runs
  before each commit.
- **CI** runs `cargo xtask check`, then fmt, clippy (`-D warnings`), tests on Windows and Linux, and
  `cargo deny check licenses`.

## Commits

- **Small commits, one logical change each:** a library, a feature, a fix, a doc update. Don't bundle a
  milestone into one commit.
- **[Conventional Commits](https://www.conventionalcommits.org/):** `type(scope): summary`, imperative and
  lower case, for example `feat(blackbox-attrib): read VPAK vaults`, `fix(world): blend SHD_ overlays`,
  `docs(formats): document scenery rotation`. Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`,
  `build`, `ci`, `chore`, `style`. The scope is the crate (`blackbox-tpk`, `nfsmw-data`, `nfsmw`) or the
  area (`formats`, `specs`, `xtask`, `ci`). Use the body for the why.
- Every commit passes `cargo xtask check` (the pre-commit hook runs it) and builds.

## Workflow

```sh
cp .env.example .env          # then edit NFSMW_GAME_DIR
cargo xtask install-hooks
cargo test --workspace        # synthetic-data tests
cargo test --release -p nfsmw-data -- --ignored   # tests against your install
cargo run --release -p nfsmw -- view-car BMWM3GTR
cargo run --release -p nfsmw -- view-world
```

- Formats are documented in [docs/formats](docs/formats) with evidence tags (**[verified]**,
  **[decomp]**, **[community]**, **[unconfirmed]**). Update the doc when a reader learns something new.
- Format crates take `&[u8]`; only `game-install` opens game files ([docs/architecture.md](docs/architecture.md)).
- Behaviour taken from decompiled code needs a spec in `docs/specs/` and a record in `docs/provenance/`.
- Tests that need the game are `#[ignore]` and read `NFSMW_GAME_DIR`; everything else uses synthetic bytes.
