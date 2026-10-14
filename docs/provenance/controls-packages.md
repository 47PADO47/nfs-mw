# Controls list packages: object GUIDs

- **Spec:** [controls-remap](../specs/controls-remap.md).
- **Sources:** the installed packages `Options_PC_Controller.fng` (140 objects) and `Pause_PC_Controller.fng`
  (116 objects), read with `nfsmw dump-screen` from the user's install (`D:\Need For Speed Most Wanted Black Edition`),
  2026-10-14. No decompiled code was read for this table; the GUIDs and positions are the package data.
- **Method:** each button is an `Image` with the `BUTTON` flag. A row's buttons and text objects share one y. In the
  pause package the titles sit at x = -289, the primary column at x = -30 and the secondary at x = 130. In the options
  package they sit at x = -298, -14 and 146. Rows are numbered from the top.
- **Use:** `crates/nfsmw/src/frontend/controls_layout.rs` holds the GUIDs as `OPTIONS` and `PAUSE`. `controls_screen.rs`
  looks them up by the package's name.

## Pause package (`Pause_PC_Controller.fng`)

| row | title (x -289) | primary button (x -30) | primary text | secondary button (x 130) | secondary text |
|---|---|---|---|---|---|
| 0 (y -48) | `0x0010597F` | `0x00105981` | `0x00105980` | `0x00105986` | `0x00105987` |
| 1 (y -18) | `0x0010597E` | `0x00105998` | `0x00105983` | `0x00105999` | `0x00105988` |
| 2 (y 12) | `0x0010597D` | `0x0010599A` | `0x00105984` | `0x0010599B` | `0x00105989` |
| 3 (y 42) | `0x0010597C` | `0x0010599C` | `0x00105985` | `0x0010599D` | `0x0010598A` |
| 4 (y 72) | `0x00105997` | `0x0010599E` | `0x00105996` | `0x0010599F` | `0x00105995` |
| 5 (y 102) | `0x0010598C` | `0x001059A0` | `0x0010598F` | `0x001059A1` | `0x00105992` |
| 6 (y 132) | `0x0010598D` | `0x001059A2` | `0x00105990` | `0x001059A3` | `0x00105993` |
| 7 (y 162) | `0x0010598E` | `0x001059A4` | `0x00105991` | `0x001059A5` | `0x00105994` |

The button names are `0x62C1BBD0` to `0x62C1BBD7` (primary) and `0x62C1BC33`, `0x62C1BC34` to `0x62C1BC3A` (secondary).
The package also holds the header texts `Primary`, `Secondary` (`0x00105982`, `0x0010598B`), the title `Keyboard
Configuration` (`0x001059AD`), and the hint `Press new button to remap this function` (`0x00105A6B`).

## Options package (`Options_PC_Controller.fng`)

Already in `controls_screen.rs` before this change, now `OPTIONS` in `controls_layout.rs`: titles `0x000E1917`,
`0x000FA36B`, `0x000FA37C`, `0x000FA388`, `0x00104073`, `0x00104077`, `0x0010407B`, `0x0010407E`; primary buttons
`0x000FA349`, `0x00104689`, `0x0010468E`, `0x00104693`, `0x0010469A`, `0x0010469F`, `0x001046A4`, `0x001046A9`;
secondary buttons `0x000FA366`, `0x001046DC`, `0x001046E4`, `0x001046ED`, `0x001046F5`, `0x001046FE`, `0x00104707`,
`0x00104710`. The text objects follow the same row order in `controls_layout.rs`.

## Limits

- The pause layout is checked against the dump's positions, not drawn. The rewrite has not run the pause package
  offscreen.
- Both packages have eight rows. The Map action (and ShiftUp) has no row (see the spec's row limit).
