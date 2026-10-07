# Provenance records

One file per crate or module whose behaviour came from a restricted source (decompiled code, GPL code,
unlicensed code). Format-only readers that just follow the docs in [`../formats`](../formats) don't need
a record. Process: [licensing.md § Spec-first](../licensing.md#spec-first).

## Records

| Module | Spec | Restricted sources read |
|---|---|---|
| [`nfsmw-compress::huff`](nfsmw-compress-huff.md) | [formats/huff.md](../formats/huff.md) | C&C Generals EAC source (GPL-3.0), dbalatoni13/nfsmw `LZCompress` (decompiled, CC0) |

## Template

```markdown
# <crate or module>

- **Spec:** docs/specs/<topic>.md (or docs/formats/<format>.md)
- **Sources read for the spec:** <project, URL, license, which files>, ...
- **Implemented:** <date>, from the spec only (decompiled code not open while writing).
- **Checked against the game by:** <tests, measurements, side-by-side captures>
- **Known differences from the original:** ...
```
