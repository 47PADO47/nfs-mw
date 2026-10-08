# Development setup

Everything you need to build, run and contribute to the project.

## Required

| Tool | Version | What it's for |
|---|---|---|
| **Rust** (stable) | latest stable | Building the project. Install via [rustup](https://rustup.rs/). |
| **NFS: Most Wanted** | PC v1.3 or Black Edition | The game install — all data is read at runtime. |

That's it for building and running the viewers.

## Recommended

| Tool | What it's for |
|---|---|
| **Python** 3.10+ | Research scripts (`tools/chunkdump.py`) and tests (`python -m unittest discover tests`). |
| **cargo-deny** | License checking (`cargo deny check licenses`). CI runs it, so install locally to catch issues early: `cargo install cargo-deny`. |

## Reverse engineering (optional)

These are only needed if you're working on format research or decompilation, not for
building the project.

| Tool | Notes |
|---|---|
| **Ghidra** 11+ | Free disassembler / decompiler. Download from [ghidra-sre.org](https://ghidra-sre.org/). Needs JDK 21+. |
| **Ghidra MCP** (`bethington/ghidra-mcp`) | Exposes Ghidra's tools over MCP so Claude can query a loaded binary. Build with Gradle, deploy into Ghidra, run the bridge with `uv`. |
| **IDA Pro / Home** (paid) | Alternative to Ghidra. Needs a paid edition for IDAPython / idalib / MCP support. IDA Free does not work with MCP. |
| **ReAgent** (`auto-re-agent`) | AI-assisted binary-to-C++ reconstruction. Uses Claude CLI as the LLM provider. Always add a behavioural differential gate (see [TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md#3-reagent-workflow-binary--cc)). |
| **MSVC** (Build Tools) | Compiling and validating reconstructed C/C++ from ReAgent. |

## First-time setup

```sh
# clone and enter the repo
git clone https://github.com/47PADO47/nfs-mw.git
cd nfs-mw

# point to your game install (pick one)
cp .env.example .env              # then edit NFSMW_GAME_DIR in the file
# or: export NFSMW_GAME_DIR="/path/to/game"
# or: nfsmw check-install prints the config.toml path you can edit

# install the pre-commit hook
cargo xtask install-hooks

# verify your install
cargo run --release -p nfsmw -- check-install

# run the tests
cargo test --workspace                                  # synthetic-data tests (no game needed)
cargo test --release -p nfsmw-data -- --ignored         # tests against your install

# launch a viewer
cargo run --release -p nfsmw -- view-car BMWM3GTR
cargo run --release -p nfsmw -- view-world
```

## Editor setup

The workspace uses `rustfmt.toml` for formatting and `.cargo/config.toml` for build
settings. Most editors with rust-analyzer will pick these up automatically.

## CI

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs on every push and PR:

1. `cargo xtask check` (leak-check + file-size check)
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace -- -D warnings`
4. `cargo test --workspace`
5. `cargo deny check licenses`

All five must pass. Run them locally before pushing.
