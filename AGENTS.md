# Agent rules

Rules that AI coding agents must follow when contributing to this project. Read
[CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor guidelines; this file
adds agent-specific rules and emphasises the style constraints agents tend to break.

## Project overview

Start here before writing any code:

- [docs/architecture.md](docs/architecture.md) — workspace layout (`libs/` vs `crates/`),
  install discovery, streaming, graphics backends, testing, roadmap.
- [docs/README.md](docs/README.md) — file format documentation index, evidence tags,
  open questions.
- [CONTRIBUTING.md](CONTRIBUTING.md) — rules, code organisation, guard rails, commit
  conventions, workflow.
- [docs/licensing.md](docs/licensing.md) — what can be ported and the spec-first process.

## Code organisation

- **Engine-generic code goes in `libs/`**, game-specific code in `crates/`. `crates/` depends
  on `libs/`, never the reverse.
- **No file over 500 lines** (source or docs). Split by domain into folders and modules, one
  struct or concern per file. `cargo xtask size-check` enforces it.
- **`libs/` stays Bevy-free, with one exception:** Bevy-based renderer backend crates are allowed in `libs/` as optional leaf crates that nothing else depends on ([ADR 0004](docs/decisions/0004-swappable-renderers.md)).
- **Format crates take `&[u8]`** and never open files. Only `game-install` touches the
  install.
- **No NFS:MW-specific names, paths or defaults in `libs/`.** Game differences go into
  per-version layout tables chosen from the data or passed in by the caller.

## Control flow: early returns, no else

Always use early returns (guard clauses) instead of `if/else` or `else if` chains.
Return, continue, or break from the failing condition as soon as possible so the
happy path stays at the lowest indentation level.

```rust
// good
fn process(x: Option<u32>) -> Result<u32, Error> {
    let x = match x {
        Some(v) => v,
        None => return Err(Error::Missing),
    };
    if x == 0 {
        return Err(Error::Zero);
    }
    Ok(x * 2)
}

// bad — nested if/else
fn process(x: Option<u32>) -> Result<u32, Error> {
    if let Some(v) = x {
        if v != 0 {
            Ok(v * 2)
        } else {
            Err(Error::Zero)
        }
    } else {
        Err(Error::Missing)
    }
}
```

This applies to all languages in the repo (Rust, Python, scripts). `match` arms
that map values are fine; the rule targets conditional blocks that decide whether
to continue or bail out.

## Guard rails

Before committing, verify all of these pass:

```sh
cargo xtask check          # leak-check + size-check
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

The `.gitignore` is a whitelist — new file types must be explicitly allowed.

## Commits

- **Small commits, one logical change each.** Don't bundle a milestone into one commit.
- **[Conventional Commits](https://www.conventionalcommits.org/):** `type(scope): summary`,
  imperative and lower case. Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`,
  `build`, `ci`, `chore`, `style`. Scope is the crate or area.

## Pull requests

When an agent opens a pull request it must follow the PR template in
`.github/PULL_REQUEST_TEMPLATE.md`. Every section must be filled in and every
checklist item must be addressed (checked or explained). Do not skip the template
or leave placeholder text.

## Issues

When an agent opens an issue it must use the matching issue template from
`.github/ISSUE_TEMPLATE/`. Pick `bug_report.md` for bugs and `feature_request.md`
for enhancements. Fill in every section.

## Tools

| Tool | Purpose |
|---|---|
| `cargo xtask check` | Leak-check (no game data, binaries, decomp output) + file-size check (500-line limit) |
| `cargo xtask install-hooks` | Install the pre-commit hook that runs `cargo xtask check` |
| `cargo deny check licenses` | Verify all dependency licenses are allowed |
| `python tools/chunkdump.py <file>` | Dump the bChunk tree of a game data file |
