# nfsmw-save-editor (x07x08)

Repository survey for the nfs-mw project. Not a clone or a port: this page records behaviour and facts only.

## 1. Summary and metadata

A browser-based save editor for the 2005 PC game Need for Speed: Most Wanted. The user opens a save file in a static web page, edits a small set of stats (money, bounty, infractions, pursuit and per-car pursuit stats, case name), and downloads a rewritten save with its MD5 checksum recomputed. No server, no install, no plugin loader.

| Field | Value |
|---|---|
| Repo | https://github.com/x07x08/nfsmw-save-editor |
| Author | x07x08 (GitHub user; no further identity stated) |
| Type | Tool (web save editor). Not an .asi plugin, not research |
| Licence | Unlicense (public domain dedication), per LICENSE file and page sidebar [source] |
| Language | Not shown on the page. Files are HTML, JavaScript and CSS [source] |
| Target build | "Tested on the 1.3 English version (6,029,312 bytes)"; README also says PS2 saves are supported [README] |
| Popularity | 30 stars, 11 forks, 21 commits on `main`, 4 pull requests [README page metadata] |
| Latest activity | No commit date visible in the captured page. Unknown |

## 2. What it does

User-visible features, each with its evidence:

- Load a save via a file picker (`#openbutton` opens a hidden `#opensave` input) [source: index.html, script.js].
- Choose platform `pc` or `ps2` from a dropdown (`#platformsList`) [source: script.js].
- Display the in-game name and the current save hash (`#playername`, `#filehash`) [source: index.html].
- Edit these values [README, Editable stats; confirmed in script.js field table]:
  - Money
  - Bounty
  - Infractions
  - Pursuit stats
  - Per-car pursuit stats (up to 6 car records, `MAX_CARS` = 6)
  - Up to 5 pursuit records (`MAX_PURSUITS` = 5)
  - Case name (max 12 bytes, zero-padded) [source: script.js]
- Save: download the edited file under the in-game name, with no extension added [source: script.js, `saveButton` handler].
- Sound and music toggle buttons (`#soundbutton`, `#musicbutton`) [source: index.html; the audio behaviour itself is not verified].

Known limitation [README]: "Can't convert the time" (time values are not editable).

## 3. How it does it

All logic is client-side JavaScript. Nothing is sent to a server.

### Loading

- A `change` listener on `#opensave` calls `FileReader.readAsArrayBuffer` [source].
- The buffer is wrapped in a `DataView` with offset 0 and an expected size of 63596 bytes for PC or 62689 bytes for PS2 [source].
- There is no magic-number, header, checksum or chunk validation. A file is accepted by length only [source].
- Multi-byte values are read little-endian by default; single-byte fields ignore the flag [source].

### Field locations (fixed offsets in the save body)

Offsets are per platform where noted. These are observed constants in the code, not documented in the README. [source]

| Field | PC offset | PS2 offset | Notes |
|---|---|---|---|
| money | 0x4039 | (table) | |
| name | 0x5A31 | (table) | 8 bytes, read only |
| caseName | 0x429D | (table) | length 0xC, zero-padded |
| cars block | 0xE2ED | 0xE329 | record size 0x38; id size 0x10; bounty at record + 0x10 |
| pursuit block | 0xF2BA | 0xF2F9 | record size per `PURSUIT_STRUCT_SIZE`; time at record + 0xC |

Chained fields compute their position as `prev.pos + BYTES_PER_ELEMENT * prev.length` [source]. Car records whose first byte is 0xFF are skipped as empty [source].

Note: the README's "1.3 English (6,029,312 bytes)" is the size of the whole file. The 63596-byte value used by the code is the expected length of the save body the editor reads. The page does not explain the relation [inferred].

### Writing

- Setters call `DataView.setUintN`, `setIntN` or `setFloatN` directly on the in-memory buffer [source].
- Each write except to the hash field calls `updateHash()` [source].
- Input values come from text fields as strings with no clamping. Out-of-range values wrap or truncate silently [source].
- `name` is decoded from the file and never written back [source].

### Checksum

- The hash is MD5, computed with the bundled js-md5 library over the save body from offset 0x34 for `size - 0x34 - 16` bytes [source].
- The 16-byte digest is stored at `size - 16` [source].
- The hash setter is guarded to avoid recursion [source].

### Export

- `saveButton` builds a `Blob` (`application/octet-stream`) and calls FileSaver.js `saveAs` [source].

### Libraries bundled (per file headers)

- js-md5, crel, FileSaver.js: MIT licences stated in their headers [source].
- Pico CSS and Eurostile Free font under `external/` [source: index.html paths; font licence not checked].

### Not present [source]

- No checksum verification on load.
- No undo.
- No bounds or range checks.
- No configuration file.
- No ASI loader, DLL, DirectX hook, game-memory access or patching. The editor works only on a copy of a save file on disk.

## 4. Configuration

None in the file format sense. The only persistent state is in script.js:

| Key | Purpose | Default |
|---|---|---|
| `playSound` | Sound toggle | Not verified |
| `playMusic` | Music toggle | Not verified |
| `currentPlatform` | `pc` or `ps2` offset table | Set from `#platformsList`. Default not verified |

Defaults were not confirmed from the captured text.

## 5. How we could implement this in nfs-mw

This repo is Unlicense, so copying would be legally permitted. Our project still works spec-first (docs/licensing.md), so we describe behaviour and derive the layout ourselves. We do not copy its code or its offset tables without independent verification against our own install.

### Behaviours to specify

1. Load a save as bytes, select a platform layout, and report whether the length matches the expected layout.
2. Read the named fields (money, bounty, infractions, pursuit stats, per-car and per-pursuit records, case name).
3. Write fields with range checks and no silent truncation. Reject out-of-range input.
4. Recompute the stored MD5 after any write, over the region the spec defines.
5. Export the edited bytes unchanged apart from the edited fields and the checksum.
6. Round-trip test: load, change nothing, save, byte-compare.

### Data and formats needed

- The save layout per platform (offsets, element sizes, record counts) must be derived from our own reading of the install, using `tools/chunkdump.py` where it applies. Do not take the offsets above as truth until verified.
- The checksum region and algorithm must be confirmed by checking that an unmodified save reproduces its stored digest.
- PS2 layout is separate and needs its own verification.

### Natural home

- NFS:MW save layout tables and field semantics: `crates/` (game-specific). A `crates/` save-format module takes `&[u8]` and never opens files, consistent with the format-crate rule in AGENTS.md. File I/O belongs to the app layer.
- Generic pieces (little-endian field codec, MD5 wrapper, bounded integer setters): `libs/` (engine-generic, no NFS:MW names). Crates depend on libs, never the reverse.
- The browser UI, if we want one, is a separate app and is out of scope for `libs/`.

### Risks

- Offsets are hard-coded and version-specific. Other builds or the PS2 format will break silently if we copy the table.
- MD5 over an unexplained region: if our region is wrong, the game may reject the save. Verify with a real game load.
- Silent wrap on out-of-range values can corrupt saves. Spec explicit bounds.
- The game may check more than the hash (not tested here).
- Editing a save can make the game unplayable or unlock states the user did not intend. Document it.

### Feature applicability

- Loader-specific hacks (ASI, DirectX) do not apply. This repo has none.
- This is the save-editing feature only. It does not touch runtime game logic, so it fits the clean-room save-format work, not the physics or AI specs.

## 6. Open questions and what could not be verified

- Latest activity date and last commit: not visible in the captured pages.
- Primary language breakdown: empty on the captured page. We infer JavaScript from the file set.
- Whether the MD5 region is exactly as stated: taken from code read via a summarising fetch, not run.
- Meaning of the 63596 and 62689 byte sizes versus the 6,029,312-byte file the README cites: unexplained.
- Default values for `playSound`, `playMusic` and `currentPlatform`: not verified.
- Whether the 1.3 build is the only tested build: README says so; no other builds listed.
- The code was read through an LLM summary of the fetched file, not by reading the raw text line by line. Offsets and constants should be re-checked against the raw source before any use.
- Not verified: that the editor works in-game, the PS2 offsets, and the font licence.
- Commit history and PR content were not read.

## 7. Sources

- https://github.com/x07x08/nfsmw-save-editor (repo page: metadata, file listing)
- https://raw.githubusercontent.com/x07x08/nfsmw-save-editor/main/README.md
- https://github.com/x07x08/nfsmw-save-editor/tree/main/src (listing: favicon.ico, index.html, script.js, style.css)
- https://github.com/x07x08/nfsmw-save-editor/tree/main/external (listing: fonts/eurostilefree, images, picocss, sounds)
- https://raw.githubusercontent.com/x07x08/nfsmw-save-editor/main/LICENSE
- https://raw.githubusercontent.com/x07x08/nfsmw-save-editor/main/src/script.js
- https://raw.githubusercontent.com/x07x08/nfsmw-save-editor/main/src/index.html
- Credit cited by the README (not read): https://github.com/BilawalAhmed0900/NFSMWSaveE
