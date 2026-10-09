# Front-end menus and FEng input (blackbox-feng, `nfsmw` frontend)

Modules: `libs/blackbox-feng` (`runtime/input.rs`, `runtime/nav.rs`, `ids.rs`: pad input, button focus and navigation)
and `crates/nfsmw/src/frontend/` (the screens, the flow, the option rows) on `crates/nfsmw/src/ui/` (packages,
assets, presenter).

- **Specs:** [specs/feng-input.md](../specs/feng-input.md), [specs/frontend-menus.md](../specs/frontend-menus.md)
  (and [specs/feng-runtime.md](../specs/feng-runtime.md) for the runtime they sit on).
- **Sources read for the specs:**
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), `src/Speed/Indep/Src/`:
    `FEng/{FEngine,FEJoyPad,FEButtonMap,FEPackage,FEObject,FEKeyTrack}`, `Frontend/{FEJoyInput,FEPackageData,
    FEPackageManager,FEManager}`, `Frontend/FEngHashes/*`, `Frontend/FEngInterfaces/FEngInterface.cpp`,
    `Frontend/MenuScreens/Common/{FEMenuScreen,feIconScrollerMenu,feUIWidgetMenu,feWidget,Slider}`,
    `Frontend/MenuScreens/Safehouse/{uiMain,options/uiOptionsMain,options/uiOptionsScreen,options/uiOptionWidgets}`,
    `Frontend/MenuScreens/InGame/uiPause`, `Frontend/MenuScreens/Loading/{FEBootFlowManager,FEMovieScreen,
    FESplashScreen}`. Read for understanding; no code copied. Constants (message ids, label hashes, repeat times,
    geometry scores) are facts about the data and behaviour.
  - The packages of the user's own install (`nfsmw dump-screen`, `nfsmw strings`): object names such as
    `OPTION_NAME_n`, `BASE_SLIDER_n`, `OPTION_n` hash to the ids found in the packages; label hashes resolve to the
    strings the specs name. The dumps are not part of the repository.
- **Implemented:** 2026-10-08, from the specs.
- **Icon transitions checked:** 2026-10-09, re-read the reference scroller's fade state, positioning and
  `EXIT_STARTED` handling, plus its header's fade-start methods. Verified the exit event at the beginning of
  the installed packages' leave scripts. Recorded the behavior in spec section 2 before implementing the
  outgoing transition independently; interrupted/repeated exit continuity is an enhancement.
  Checked elapsed timing at 30/60/120 Hz, native accept/back/start exits on base and modded menu packages,
  Controls exit/return, and settled/intermediate/outgoing frames rendered by the release executable.
- **Checked against the game by:** unit tests on synthetic packages (messages, repeat, navigation), the ids test
  (every message id equals the hash of its name), and screenshots of the screens run from the install. Not
  compared with the running original.
- **Known differences from the original:**
  - Wrapping of the geometric navigation (`Wrap_Horizontal` / `Wrap_Vertical`) is not implemented; the original
    default is no wrapping and no screen here asks for it.
  - One pad: the original has one mask per controller and per-package control masks; here a package has control
    or not.
  - Sounds the screens send (`SOUND_*` messages) are reported to the host and ignored.
  - The icon-menu scroll uses a cubic ease of 0.2 s; the original seeks a spline whose exact shape was not read.
  - Dialogs (confirm on leaving a changed option screen, defaults), the keyboard screen and the memory-card
    screens are not implemented.
  - The option rows are those of this rewrite's settings (spec section 3.1), not the original settings.
