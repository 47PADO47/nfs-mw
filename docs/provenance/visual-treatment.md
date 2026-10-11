# Visual Treatment provenance

Date: 2026-10-11. Specification: [Visual Treatment](../specs/visual-treatment.md).

Original PC shader bytecode and runtime constants were examined privately. The
luminance extraction, four-tap blur and colour composition were described in the
specification before independently implementing Rust/WGSL. Authored curves and
tints are read from the user's installed AttribSys data, never redistributed.

References: dbalatoni13/nfsmw (CC0, symbols); magcius/noclip.website (MIT,
reverse-engineered post-processing). No source copied from either project.

Validation records belong in [the feature guide](../visual-treatment.md). Exact PC
dynamic exposure and time-of-day/pursuit look parity remain unverified. The
quarter-size targets are an implementation choice, not full engine parity.
