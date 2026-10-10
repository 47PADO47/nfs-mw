# Licensing, credits and provenance

How this project stays publishable: what we may take from other projects, how we credit them, and how
behaviour learned from decompiled code gets into the Rust code without copying it.

## The project's license

The project is licensed **`MIT OR Apache-2.0`** ([LICENSE-MIT](../LICENSE-MIT),
[LICENSE-APACHE](../LICENSE-APACHE)). Contributions are accepted under the same terms. Third-party sources
we learned from are credited in [NOTICE](../NOTICE).

## Hard rules

1. **No game assets in the repo.** This covers game files, extracted textures, models, audio and video, and
   dumps of them. The game is read at runtime from the user's own install. Hashes, sizes, offsets and
   counts measured on the install are fine.
2. **No decompiled code in the repo**, whoever decompiled it and whatever license it carries. That
   includes the dbalatoni13 decomp, Ghidra/IDA output and "decomp" folders in other projects.
3. **No code copied from GPL, LGPL, AGPL or unlicensed sources.**
4. **Credit everything we learn from**, in NOTICE and in the relevant doc, with its URL and license.

The whitelist [`.gitignore`](../.gitignore), the leak check (`cargo xtask leak-check`, run by the
pre-commit hook and CI) and code review enforce rules 1 and 2. See [CONTRIBUTING.md](../CONTRIBUTING.md).

## What each kind of source allows

| Source license | Read and learn from it | Copy or port its code | Use it as a dependency | What we must do |
|---|---|---|---|---|
| MIT, BSD, ISC, Apache-2.0, Zlib, BSL-1.0 | yes | yes | yes | Keep its copyright and license notice at the top of the ported file, and list it in NOTICE |
| CC0, Unlicense, public domain | yes | yes | yes | Credit it in NOTICE (courtesy, not a legal requirement) |
| MPL-2.0 | yes | only as whole files that stay MPL-2.0 | yes | Don't modify MPL files inside this repo |
| LGPL | yes | no | no (static linking into a Rust binary brings relinking obligations) | — |
| GPL, AGPL | yes, for understanding | no | no | Describe the algorithm in our own words if we need it (see below) |
| **No license** | yes, as a reference | **no** | no | See next section |
| Decompiled code (any label) | yes | no | no | Spec-first (see below) |

### Code ported under these rules

NOTICE lists each project with the files that carry its notice: VaultLib (AttribSys), vgmstream and utkencode
(EA audio codecs), and AMD's FidelityFX Super Resolution 1 (MIT), whose EASU and RCAS were rewritten in WGSL in
`libs/blackbox-render/src/shaders/fsr1.wgsl` with AMD's and Michal Drobot's notices at the top of the shader.
Upstream headers are read from the scratchpad only and never committed.

### Repositories with no license

A repository without a license is **not** MIT. Under copyright law (the Berne Convention, which nearly
every country follows) code is protected automatically when it is written. Without a license, nobody
else has permission to copy, modify or redistribute it, even with credit. GitHub's terms of service only
let other users view and fork it on GitHub.

So for an unlicensed repo:

- We read it, learn from it, and document the **facts** it reveals (offsets, field layouts, chunk IDs,
  magic numbers, hash algorithms) with credit. Those are not copyrightable expression.
- We **don't copy its code**. To use its code, ask the author to add a license (MIT works for us),
  for example by opening an issue. Then record the permission or the new license in NOTICE.

Unlicensed projects we rely on today: NFS-ModTools, FEngLib, Attribulator, Brawltendo's vehicle decomp,
nfsu2-re, NFSMWUnlimiter, MWEncyclopedia, and parts of the NFSTools org. All are used as references only.

## Facts versus code

Every format doc in [`formats/`](formats) records **facts**: layouts, offsets, IDs, value ranges. They are
measured on the install (**[verified]**) or taken from a source and credited (**[decomp]**,
**[community]**). Facts can come from any source, including decompilations and unlicensed tools. Code,
meaning the expression, comes only from permissive sources.

## Spec-first

Behaviour with no other source than decompiled code (car physics, AI, pursuit, the streaming logic)
goes through a written specification first:

1. **Read** the decomp (or other restricted source) and write `docs/specs/<topic>.md`. Describe what the
   code does in plain language, math and pseudocode: inputs, outputs, state machines, formulas and
   constants. Don't translate line by line. Tuning numbers come from AttribSys data, not from code.
2. **List the sources read** at the top of the spec, with their licenses.
3. **Implement** the Rust code from the spec, without the decompiled code open. Ideally a different
   session or person does this.
4. **Record provenance** in `docs/provenance/<crate-or-module>.md` (template in
   [provenance/README.md](provenance/README.md)): the spec, the sources, the date, and how the result was
   checked against the real game.

The same process applies to algorithms whose only reference implementation is GPL. For example, the
HUFF decoder was written from [formats/huff.md](formats/huff.md).

## About the decompilation's request

The dbalatoni13/nfsmw README asks people not to build ports on the unfinished decomp, especially with
AI ("SAY NO TO SLOP"). The decomp is CC0, so this is a request, not a license term. The project owner has
read it and chose to proceed. We use the decomp as a read-only reference under the rules above and credit
it in NOTICE.

## Dependencies

- [`deny.toml`](../deny.toml) allows only permissive licenses plus MPL-2.0; `cargo deny check licenses`
  runs in CI.
- Before a binary release, generate third-party notices with `cargo about` from `Cargo.lock` and ship them
  with the binary (planned).

## Trademarks

*Need for Speed* and *Most Wanted* are trademarks of Electronic Arts Inc. This project is not affiliated
with or endorsed by Electronic Arts. You need your own copy of the game to use it.
