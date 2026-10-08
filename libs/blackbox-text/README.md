# blackbox-text

Reader for the language string tables of EA Black Box games (`LANGUAGES/English.bin` and friends in Need for
Speed: Most Wanted): a chunk `0x00039000` holding a sorted table of `{label hash, packed string}` and the
histogram that unpacks the strings.

```rust
let table = blackbox_text::StringTable::from_file(&bytes)?;
assert_eq!(table.get(blackbox_text::label_hash("SOME_LABEL")), Some("...".to_string()));
```

Takes bytes, never opens files. Format: `docs/formats/text.md`.

License: MIT OR Apache-2.0.
