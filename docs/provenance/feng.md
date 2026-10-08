# FEng packages, fonts and runtime (blackbox-feng)

Modules: [`libs/blackbox-feng`](../../libs/blackbox-feng) (package and font readers, the script and message
runtime, the retained `UiTree`) and [`libs/blackbox-text`](../../libs/blackbox-text) (language string tables).
The game side is `crates/nfsmw/src/hud/`.

- **Specs:** [formats/frontend.md](../formats/frontend.md) (layouts),
  [specs/feng-runtime.md](../specs/feng-runtime.md) (runtime and drawing rules),
  [formats/text.md](../formats/text.md) (string tables).
- **Sources read for the specs:**
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled): `src/Speed/Indep/Src/FEng/*`
    (`FEngine`, `FEPackage`, `FEObject`, `FEScript`, `FEKeyTrack`, `FEKeyInterp*`, `FEMessageResponse`,
    `FEMsgTargetList`, `FEEvent`, `FEPackageReader`, `FEPackageChunks.h`), `Frontend/{FEngRender,FERenderObject,
    FEngFont,RealFontOld}`, `FEngInterfaces/FEGameInterface.cpp`, `HUD/*`, `Localization/*`. Read for
    understanding; no code copied. Constants (tag ids, message ids, font offsets) are facts.
  - [NFSTools/FEngLib](https://github.com/NFSTools/FEngLib) has **no license**: used only to cross-check facts, no code
    taken.
  - Probes of the user's own install (210 distinct packages, five fonts, the HUD texture packs): the facts marked
    **[verified]** in the format document. The probe programs are not part of the repository.
- **Implemented:** 2026-10-08, from the specs only (the decompiled code was not open while writing).
- **Checked against the game by:** unit tests on synthetic packages, and `#[ignore]` tests that load
  `HUD_SingleRace.fng` and the fonts from an install. Not compared with a capture of the original HUD yet.
- **Known differences from the original:**
  - Package commands (switch, push, pop) are reported to the host instead of executed; the host owns the package list.
  - Multi images draw without the mask.
  - List boxes, code list boxes, movies and the mouse are not implemented (none occur in the HUD, no list type
    occurs in any package).
  - The tick rate (960 per second) is inferred, not read.
