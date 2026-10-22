# Mod save data

Mods need two kinds of save access. They keep their own data per career profile, such as a car collection or a score
table, in a separate file (sections 2 to 7). They also read the game's save data and write to it, such as the cash or
a career flag (section 9). This spec is the design for both: where the data lives, the interfaces a mod calls, the file
format, and what happens on save, load and delete. It builds on [ADR 0005](../decisions/0005-native-mod-plugins.md) (mods, the ABI
and the interfaces) and on [career-save.md](../formats/career-save.md) (the profile file it must leave alone).

**Status.** Built: the blob file and its checks (`crates/nfsmw/src/career/mod_data.rs`, sections 4 and 3's key and size
rules), and the delete of a profile's mod data (section 2). Not built yet: the `bb.saves` and `nfsmw.career` interfaces
(sections 3 and 9), the save-path write order (section 5), and the decisions in section 8.

This is a design of this project, not a behaviour of the original game. The original game reads only its own profile
file and never looks at the files this spec adds.

## 1. Why not inside the profile

The profile buffer is a fixed 0xF848 bytes, and the load checks reject any other size (`career-save.md` section 1 and
the game check of section 2.2). Adding bytes would make the file unreadable by the original and by this build. The
unknown bytes inside the existing blocks are kept, but the original may use them, so a mod must not write there.
Changes to the game's known fields go through the career's own methods (section 9), never through raw offsets.

## 2. Where the data lives

Mod data is written to this build's own profile folder only, next to the profile file:

```
config\career\<alias>\<alias>          the profile (unchanged)
config\career\<alias>\mods\<mod-id>.bin one file per mod that has data for this profile
```

- The original's folder (`Documents\NFS Most Wanted`) is read only, as in `career/sources.rs`. Mod data is never
  written there.
- A profile loaded from the original has no `mods` folder until a mod writes. Its first save copies the profile file
  into the own folder (the existing rule). Mod data written before that first save goes into the own folder too; it is
  not copied from the original, because the original has none.
- The alias folder is the unit of deletion. `ProfileDir::remove` (`career/io.rs`) deletes the profile file and then
  removes the folder only when it is empty. With a `mods` subfolder that would leave the folder behind, and a new
  profile with the same name would inherit the old mod data. So the delete must also remove the `mods` folder and its
  files. This is a change the implementation has to make; the current code does not do it.

## 3. The interface a mod calls

The interface is `bb.saves`, a generic interface in `blackbox-modding` (game-independent, ADR 0005 section 8). The game
implements it for the loaded career profile. It is added to the interface list in ADR 0005 as an amendment; this spec
does not change the ADR's decisions.

| Call | Meaning |
|---|---|
| `read(key) -> bytes or none` | the mod's blob for the loaded profile; none when the mod has not written one |
| `write(key, bytes)` | replace the mod's blob; written with the next profile save or at once, see section 5 |
| `remove(key)` | delete the mod's blob |

- `key` is the mod's own id from `mod.toml` (`[a-z0-9_.-]`, lower case, up to 64 characters). The host never builds a
  path from it except after checking it against that pattern, so it cannot reach outside the `mods` folder.
- The bytes cross the ABI as a pointer and a `u64` length (ADR 0005 section 1). `bytes` is opaque to the host.
- With no career profile loaded, every call fails with a status code; a mod that needs a profile checks first.
- A blob is at most **256 KiB** (proposal). A larger write fails with a status code and changes nothing.
- The calls are refused in an online session when the career gameplay interface is switched off (ADR 0005 section 6).
  Saves are local, so this does not affect what a session sends.

## 4. The file

Each `<mod-id>.bin` is:

| Offset | Size | Field |
|---|---|---|
| 0x00 | 16 | MD5 of the rest of the file (`save-store::digest`, `with_prefix`) |
| 0x10 | 4 | magic `MODD` |
| 0x14 | 4 | format version, `1` |
| 0x18 | 8 | length of the body in bytes |
| 0x20 | n | body: the mod's own bytes, opaque to the host |

The MD5 is the same integrity check the save-store uses. It catches a torn or corrupted file; it is not a secret and
not a security boundary (mods are trusted code, ADR 0005 section 6). The mod owns the layout of its body, including its
own version number.

A file that fails the check, the magic or the length is **corrupt**. A read of a corrupt blob returns a distinct status
code, so the mod can react, and the file is left in place. A later write replaces it.

## 5. Save, load and delete

- **Write.** `write` stores the blob in memory until the next profile save. A profile save (manual, or autosave) first
  writes every changed blob, then the profile file. The profile file is the commit point: if a blob write fails, the
  profile save stops and the profile is not written, so the mod data and the profile never go out of step.
- **Write-through option.** A mod that needs its data on disk before the next save may ask for an immediate write. This
  is the same atomic write (temporary file, then rename) as the profile, so a crash leaves the old blob whole.
- **Read.** `read` loads the file the first time it is asked, when the profile is loaded. A profile with no `mods`
  folder gives none for every key.
- **Remove.** `remove` deletes the file at the next profile save, not at once, so a save that fails leaves the old blob.
- **Deleting a profile** removes its folder (section 2). No blob survives a deleted profile.
- **Uninstalled mods.** A blob whose mod is no longer installed stays on disk. Nothing reads it, and the player can delete
  it by hand. The host does not clean it up.
- **Copying a profile.** The files are plain. Copying the alias folder from the own folder to another machine carries the
  mod data with it, as long as the same mods are installed there.

## 6. Interaction with the existing code

- `career/io.rs` owns the profile folder. The mod data is a `mods` subfolder of the same alias folder, so the code for
  the alias folder (`ProfileDir`) gains a helper for it. The write order in section 5 is in the profile save path.
- The file format and the MD5 come from `libs/save-store` (`digest::with_prefix`, `digest::check_prefixed`). The slot
  name check in `save-store` does not accept dots, and mod ids do. Either the check is extended for mod keys or the
  mod data uses its own name check; this spec leaves that to the implementation, with the rule in section 3.
- The interface is a host table in `blackbox-modding` (`abi` feature for the table, `host` for the dispatch), with the
  game's side in `crates/nfsmw` and the save path in `crates/nfsmw-career`. `libs/` stays free of NFS:MW names (AGENTS.md).

## 7. Not in this design

- Sharing data between mods. Each key belongs to one mod; a mod that wants another's data must go through that mod's
  own interface.
- Encryption or signing. The files are readable by the user, as the mods are.
- Making the original game read the data. It cannot, and it need not.
- Migration of data written by an older mod version. The mod owns its version field (section 4).
- Data outside a career profile, such as global settings. Those stay in `config.toml` (ADR 0005, `bb.config`).
- Raw byte access to the profile buffer. Mods see only the typed fields of section 9.

## 8. Open decisions

1. Accept the own-folder-only location (section 2), or also allow an option to store mod data beside the original's
   folder. The proposal is own folder only, because the original's folder is never written.
2. The blob size limit of 256 KiB (section 3).
3. Whether a mod may write immediately (section 5, write-through) in the first version, or only with the profile save.
   The proposal is to include it, since it uses the same atomic write.
4. Whether to amend ADR 0005 with `bb.saves` now or with the first implementation.
5. Whether a deleted profile's mod data is removed with it (section 2). The proposal is yes: the delete removes the
   `mods` folder too, so a new profile with the same name starts empty.
6. The first list of game fields that mods may read and write (section 9). The proposal is the operations the game
   already performs: reading cash, stage and career flags, and writing cash through `add_cash` and rewards through
   `grant_rewards`. Nothing else until a mod needs it.
7. Whether a save records which mods changed its data. A save that a content mod has changed may fail the stable-hash
   check without that mod (section 9.4). The proposal is to record the mod ids in the mod's own file, so the player can
   see what a save needs.

## 9. Game save data

Mods read and write the profile through the `nfsmw.career` interface, the game-specific interface of ADR 0005. The
calls are typed: a mod gets a field value, or asks the career to do one of the operations the game already performs. The
host implements them on `CareerHandle::with_loaded` (`crates/nfsmw/src/career`), so the same rules apply to a mod as to
the game's own code.

### 9.1 Reads

- A read returns a typed value for a field the rewrite models, such as the cash, the career stage, or a career flag.
  The fields are listed in the interface table, one per field, with its type and range.
- Reads are allowed in an online session. Nothing a read returns is sent to a peer.
- A field the rewrite does not model is not readable. The unknown bytes inside the blocks (`career-save.md`) are never
  exposed.

### 9.2 Writes

- A write calls the career's own method for that field, such as `settings.add_cash`, which already keeps the value in
  its range. No write takes an offset or a byte string.
- A write changes the career in memory. It is saved with the next profile save, in the order of section 5, so a failed
  write of a mod blob still stops the profile save.
- Writes are refused in an online session, unless the host and every client enable mods for gameplay interfaces
  (ADR 0005 section 6). Writes are not sent to a peer; a server validates results itself (ADR 0005 section 6).

### 9.3 Rules the game keeps

- The profile MD5 and the container checksums are recomputed on every save by the existing code
  (`crates/nfsmw-career/src/save/profile.rs`), so a mod cannot leave the file with a stale checksum.
- The flags merge rule of `career-save.md` section 2.3 applies to any flag write.
- A value outside the field's range is refused at the call, with a status code, and nothing changes.

### 9.4 Compatibility

- The default stable hash (`career-save.md` section 2.2) is computed from the installed car data. A mod that adds cars
  to the stable, or changes the car data, makes a save written with it fail the hash check in an install without that
  mod, and in the original game. A mod that only changes cash, flags or its own file does not change the hash.
- A save with mod-added content therefore loads only with the same mods installed. This is the existing load rule, not a
  new one. Open decision 7 covers recording it.
- The original game reads a save written by a mod only when the car data matches, by the same hash check
  (`career-save.md` section 2.2).
