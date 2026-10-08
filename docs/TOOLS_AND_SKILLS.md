# Tools & Skills — NFS: Most Wanted RE/Modding Environment

Reference for the reverse-engineering and modding toolchain available to this
project, plus the Claude Code skills, agents, and MCP servers worth reaching for.

- **Project:** Need for Speed: Most Wanted (2005) data-format RE (`.BUN`/`.BIN`/`.LZC` bChunk trees; JDLZ/RAWW compression).
- **Host:** Windows 10.
- **Last updated:** 2026-10-07.

---

## 1. Project tools (this repo)

| Tool | Purpose |
|---|---|
| [`tools/chunkdump.py`](../tools/chunkdump.py) | Dump the bChunk tree of MW data files. Handles whole-file JDLZ/RAWW decompression, then walks the `u32 id / u32 size / payload` chunk structure (bit 31 of `id` = container). Stdlib only, Python 3.10+. |
| [`tools/bchunk_names.py`](../tools/bchunk_names.py) | Known bChunk ID → name map (sourced from the `dbalatoni13/nfsmw` decomp symbols, CC0). Used by `chunkdump.py`. |
| [`tests/test_chunkdump.py`](../tests/test_chunkdump.py) | Unit tests over small synthetic files (no game data needed). Run: `python -m unittest discover tests`. |
| [`crates/`](../crates) | The Rust rewrite: `nfsmw check-install / list-cars / view-car`. See [architecture.md](architecture.md). |
| [`xtask/`](../xtask) | `cargo xtask leak-check` (no game data, binaries or decompiler output in the repo) and `cargo xtask install-hooks`. |

**Run the dumper:**
```bash
python tools/chunkdump.py <file.BUN|.BIN|.LZC>
```

---

## 2. Installed RE toolchain (the workstation)

| Tool | Version / location | What it's for |
|---|---|---|
| **Ghidra** | 12.1.4 → `D:\retoolkit\decompilers\ghidra_12.1.4_PUBLIC` (runs on JDK 25 via `JAVA_HOME_OVERRIDE` in its `support\launch.properties`). Old 10.1.3 at `D:\retoolkit\decompilers\ghidra`. | Disassembly, decompilation, headless analysis/export of binaries. |
| **JDK** | 25 → `C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot` (also JDK 17). | Runs Ghidra 12 (needs 21+) and Gradle/Java builds. |
| **MSVC** | `cl.exe` 19.51 (VS Build Tools 2026) → `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`. Not on PATH; load via `VC\Auxiliary\Build\vcvars64.bat`. | Compile/validate reconstructed C/C++. |
| **Python** | 3.14.6 (system, `C:\Python314`). | Project scripts, ReAgent, Ghidra export bridge. |
| **uv / uvx** | On PATH. | Fast Python env/runner; drives the Ghidra MCP bridge and HCLI. |
| **ReAgent** | `auto-re-agent` 0.4.0 (`re-agent`, `ghidra-bridge` CLIs). | AI-assisted binary→C/C++ reconstruction with a bounded validation pipeline. See §3. |
| **Ghidra MCP** | `bethington/ghidra-mcp` v7.0.0 (built w/ Gradle+JDK25, deployed into Ghidra, bridge `.venv` via uv). | Exposes 209 Ghidra tools to Claude over MCP. See §4. |

> **Ghidra headless on this box:** `analyzeHeadless` needs the project dir to pre-exist,
> and the `ghidra-bridge export` step needs `pyghidra` + `JAVA_HOME=…jdk-25…`.

### IDA — not usable for MCP here
- `D:\IDA Freeware 8.0` and `C:\Program Files\IDA Free 9.4` are both **IDA Free** and have **no IDAPython / no `idalib`**.
- Both the official Hex-Rays IDA MCP and `mrexodia/ida-pro-mcp` require `idalib` + a **paid edition** (IDA Home/Pro/Classroom 9.4+). IDA Free is explicitly unsupported.
- To get an IDA MCP: install IDA Home/Pro/Classroom (idalib auto-activates on 9.4+ install) → `uvx ida-hcli mcp install`. Otherwise use the Ghidra MCP, which is the free equivalent.

---

## 3. ReAgent workflow (binary → C/C++)

Config lives in `reagent-demo/re-agent.yaml`. Provider is **`claude-cli`** (uses the local
authenticated `claude` CLI — no API key; cost = whatever your Claude plan charges). Call
budget is capped small: `max_llm_calls_per_function: 8`, `max_attempts_per_function: 1`,
`max_review_rounds: 2`.

**Pipeline:** build/import binary in Ghidra → `ghidra-bridge export all` → `re-agent reverse --address <addr>`.

```bash
# from reagent-demo/, with re-agent on PATH
re-agent doctor                      # must report READY: True
re-agent reverse --address 0x140001000
```

### ⚠️ Validation is only as strong as its gates
ReAgent's default gates are **non-behavioral**: the LLM self-check, the structural
verifier, a **compile-only** build gate, and parity. **None execute the code**, so they can
pass semantically wrong output. In this project's demo, ReAgent confidently reported a PASS
on a function whose even/odd branch was *inverted* (returned `+7` where the truth was `-7`).

**Always add a behavioral differential gate** (compile + run the candidate against ground
truth). Wired in `reagent-demo/`:
- `diff_ref.py` — ground-truth harness: reads a case as JSON on stdin → true result as JSON.
- `diff_cand_wrapper.cpp` + `diff_cand.bat` — MSVC-compile the generated `{candidate_file}` and run it per case.
- `diff_cases.json` — the test cases.
- `re-agent.yaml` → `validation.differential_{cases_file,reference,candidate}`.

Gotchas: `differential_reference`/`differential_candidate` must be **flat arg lists** (not
list-of-lists); harnesses must print **only JSON** on stdout (silence `cl` to `nul`); string
build commands run via `/bin/sh` (absent on Windows) so use **argv lists**. With the gate on,
the inverted reconstruction correctly failed: `{"candidate":7,"case":[1,13],"reference":-7}`.

---

## 4. Ghidra MCP (use Ghidra from Claude)

Registered with Claude Code at **user scope** (all projects):
```bash
claude mcp add -s user ghidra-mcp --env GHIDRA_MCP_URL=http://127.0.0.1:8089 -- \
  uv run --directory /path/to/ghidra-mcp bridge-mcp-ghidra --transport stdio
```
Tools appear as `mcp__ghidra-mcp__*` (209 of them: decompile, rename, structs, xrefs, strings, etc.).

**To make calls return data:** launch Ghidra → open a program → **Tools ▸ GhidraMCP ▸ Start MCP Server** (HTTP `127.0.0.1:8089`). Until then the bridge is "Connected" but has no program to talk to.

Build/deploy (Gradle default, no Maven needed):
```bash
# JAVA_HOME = JDK 25; Git Bash needs forward slashes in the path
.\gradlew.bat buildExtension "-PGHIDRA_INSTALL_DIR=D:/retoolkit/decompilers/ghidra_12.1.4_PUBLIC"
.\gradlew.bat deploy         "-PGHIDRA_INSTALL_DIR=D:/retoolkit/decompilers/ghidra_12.1.4_PUBLIC"
```

---

## 5. Claude Code skills, agents & MCP servers

The full catalog in this session is very large (hundreds of skills across unrelated domains —
Shopify, Expo, Vercel, finance, sales, etc.). Below is the curated subset actually useful for
RE/modding and dev on this project. Invoke a skill with `/<name>`; launch an agent via the
Agent tool.

### Most relevant — game modding / RE (`universal-modder` plugin)
| Skill | Use |
|---|---|
| `universal-modder:reverse-engineering` | RE workflow guidance for game binaries/formats. |
| `universal-modder:game-recon` | Reconnaissance of a game's files, engine, formats. |
| `universal-modder:mod-any-game` | General modding playbook. |
| `universal-modder:asset-pipeline` | Extract/convert/repack game assets. |
| `universal-modder:game-automation` | Automate a running game for testing. |
| `universal-modder:game-research-websearch` | Research a game's internals from public sources. |
| `universal-modder:mashup-mods`, `:publish-mod`, `:showcase-video`, `:share-field-notes`, `:fal-assets` | Compose/ship/demo mods; generate assets. |

### General engineering skills
| Skill | Use |
|---|---|
| `diagnosing-bugs` | Structured loop for hard bugs / regressions. |
| `tdd` | Test-first feature/bugfix workflow (fits `tests/`). |
| `code-review` / `/code-review` | Review changes (standards + spec). `ultra` form runs a cloud multi-agent review. |
| `security-review` | Security-focused review pass. |
| `simplify` | Reuse/simplify/efficiency cleanups on changed code. |
| `domain-modeling`, `codebase-design` | Sharpen module boundaries & project vocabulary. |
| `research` | Deep multi-source research spike. |
| `writing-for-agents` | Author docs/prompts meant for agents. |
| `update-config` | Edit Claude Code `settings.json` (permissions, hooks, env). |
| `loop`, `schedule` | Run a task on an interval / schedule a cloud agent. |
| `claude-api` | Reference for the Claude API/SDK (models, pricing, tools). |

### Agents (via the Agent tool)
- `Explore` — read-only fan-out search across the repo.
- `Plan` — design an implementation plan.
- `general-purpose` — multi-step research/execution.
- `agent-protocols:code-reviewer`, `:security-auditor`, `:test-engineer`, `:performance-engineer`, `:documentation-specialist` — specialist review/authoring.

### MCP servers
- **`ghidra-mcp`** — ours; Ghidra over MCP (see §4). Active when Ghidra + its HTTP server are running.
- **Fiddler** (`fiddler-*` skills) — HTTP traffic capture/debugging; server currently not connected (start Fiddler Everywhere + its MCP server to use).
- Various first-party connectors (Drive, Gmail, Calendar, GitHub, etc.) exist but need authorization via `/mcp` or claude.ai connector settings.

---

## 6. Quick-start recipes

**Inspect a game data file's chunk tree:**
```bash
python tools/chunkdump.py path\to\TRACKS\L2RA.BUN
```

**Reverse a function from a game binary with ReAgent (+ behavioral gate):**
1. In Ghidra, import + auto-analyze the binary (`analyzeHeadless <proj_dir> <name> -import <bin> -overwrite`).
2. `ghidra-bridge export all` (set `JAVA_HOME` to JDK 25, `ghidra-bridge.yaml` pointing at the project).
3. Add differential cases + reference/candidate harnesses to `re-agent.yaml` (pattern in `reagent-demo/`).
4. `re-agent reverse --address <addr>` → trust the result only if `validation_verdict` includes a passing **differential** check.

**Use Ghidra interactively from Claude:** start Ghidra + GhidraMCP server, then ask Claude to use `mcp__ghidra-mcp__*` tools against the open program.

---

## 7. Known limitations / caveats
- **Python 3.14** is newer than some RE tools expect (e.g. IDA 9.4 binds ≤3.12; some wheels lag). Keep a 3.12 around if a tool refuses 3.14.
- **ReAgent PASS ≠ correct** without a behavioral/differential gate (see §3).
- **IDA Free cannot host an MCP** — needs a paid edition + idalib (see §2).
- **Ghidra MCP** returns data only while Ghidra is running with a program open and the HTTP server started.
