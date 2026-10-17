# Profiling and build profiles

General profiling notes. The Bevy renderer's Tracy workflow is in [profiling-bevy.md](profiling-bevy.md).

## Build profiles

The root `Cargo.toml` defines these profiles. Pick by what you are measuring.

| Profile | Command | Use it for |
|---|---|---|
| `release` | `cargo build --release -p nfsmw` | Everyday timing. |
| `profiling` | `cargo build --profile profiling -p nfsmw` | Profiling. Same codegen as `release` plus line tables, so Tracy, VTune, Superluminal and `perf` show function and line names. Output: `target/profiling/`. |
| `dist` | `cargo build --profile dist -p nfsmw` | Shipped builds. Fat LTO, one codegen unit. Much slower to build; use it for releases, not iteration. Output: `target/dist/`. |

`profiling` adds debug info only, so its timings match `release`. Compare numbers across the same profile.

## Allocator: `mimalloc` feature

`nfsmw` can use [mimalloc](https://crates.io/crates/mimalloc) (MIT) as its global allocator. It is behind the
`mimalloc` cargo feature and is off by default. Enable it to A/B the allocator against the system one:

```bash
cargo build --release --features nfsmw/mimalloc
```

The allocator is set in `crates/nfsmw/src/main.rs` with `#[global_allocator]`. Compare the same scene with and
without the feature under the same profile before changing the default.
