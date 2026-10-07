# blackbox-attrib

Reader for **AttribSys** gameplay databases: `VPAK` packs of vaults (`.vlt` structure + `.bin`
payload), optionally RAWW/JDLZ/HUFF-wrapped, or a bare vault pair. It resolves `PtrN` pointer fix-ups,
reads classes (typed fields) and collections (instances that inherit unset values from a parent), and
decodes values: numbers, `Text`, `StringKey`, `RefSpec`, vectors, `Blob` (with decompression) and fixed
arrays; game-specific types come back as raw bytes. Names are keyed by the lookup2 hash (`vlt_hash`), and
a `Names` dictionary maps hashes back to the strings found in the files or given by the caller.

Record layouts are per AttribSys generation (`src/layout/`) and detected per vault: the legacy layout
(NFS: Most Wanted, 2005) is implemented; other generations fail with `Error::UnsupportedLayout`.

Spec: `docs/formats/attributes.md`.

Parts are ported from [VaultLib](https://github.com/NFSTools/VaultLib) (MIT, Copyright (c) 2019 NFS Tools &
heyitsleo); those files keep its notice, and NOTICE lists it.

License: MIT OR Apache-2.0.
