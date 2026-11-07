# Mod manager icon

`mod-manager.png` is the picture of the Mods category in the options: a white puzzle piece with a switch cut out of
its body, 128 by 128 pixels, RGBA on transparency.

Provenance: drawn for this project by a script that uses only the Python standard library (no image library, no
external artwork, no game file). Its licence is the repository's own (MIT OR Apache-2.0, see `LICENSE-MIT` and
`LICENSE-APACHE`).

The image is embedded in the executable (`crates/nfsmw/src/ui/mod_icon.rs`) and requires no game installation.
Its fingerprint is pinned in `xtask/src/leak.rs` (`REVIEWED_MEDIA`): replacing the file means reviewing the new
artwork's source and licence, and updating that fingerprint.

# Rider gear icon

`rider-gear.png` is the picture of the Rider entries of the garage's Visual category (Rider Body, Helmet, Suit, Gloves, Boots): a
white full-face helmet seen from the side, 128 by 128 pixels, RGBA on transparency.

Provenance: drawn for this project by a Pillow script (ellipse, rounded rectangles and a cut-out visor, drawn at eight times the size
and scaled down; no external artwork, no game file). Its licence is the repository's own (MIT OR Apache-2.0, see `LICENSE-MIT` and
`LICENSE-APACHE`).

The image is embedded in the executable (`crates/nfsmw/src/ui/rider_icon.rs`) and requires no game installation. Its fingerprint is
pinned in `xtask/src/leak.rs` (`REVIEWED_MEDIA`).
