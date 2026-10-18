# 0008 — Themeable UI: one component system, switched by `UiStyle`

- **Status:** accepted (2026-10-18). Built through T12; the GPU parity checks are owed (see the compose spec).
- **Milestone:** cross-cutting. Touches 5 (the HUD), 6 (the menus) and 12 (the mod hook). The work is split into
  subtasks T1 to T12 in the plan (`C:\Users\PADO\.claude\plans\i-want-you-to-serialized-lantern.md`, section
  "Subtask breakdown").
- **Spec:** [docs/specs/ui-themes.md](../specs/ui-themes.md) holds the component model, the slot catalogue and the
  resolver in detail. This ADR records the decision only.

## Question

Today one element can switch between two looks: the radio announcement. `settings.radio_hud` picks the original
EA Trax chyron or the rewrite's own card. The switch is expressed in two render pipelines. The chyron goes
through FEng (`crates/nfsmw/src/hud/plugin.rs`, the `hud.runtime.tree(trax.package())` path). The card goes
through egui (`crates/nfsmw/src/devtools/mod.rs`, around lines 73 to 80, which calls `crate::hud::show_radio`).
Everything else (tachometer, speedometer, minimap, pursuit board, blips, and the whole menu, settings and row
stack) exists in original form only, drawn from the game's `.fng` packages.

How do we let the whole UI (HUD and menus) have more than one look, let a theme replace single elements, and
let a mod add its own theme or element overrides, without a second draw path? [ADR 0002](0002-ui-presentation.md)
requires exactly one presenter, and that must stay true.

## Decision

**Turn the one-off radio flag into a general UI theme switch, backed by a component system in a new
engine-generic crate. Every theme, including the original look, produces the same neutral draw tree, and the
existing presenter draws it.**

1. **A dedicated `UiStyle` setting**, `enum UiStyle { Original, Custom, Remastered }`, default `Original`. It is
   independent of the graphics `VisualStyle` (`crates/nfsmw/src/settings/appearance.rs`), because the look of
   the UI and the look of the graphics are separate choices. `radio_hud` is folded into it: the old key still
   reads (`custom` maps to `UiStyle::Custom`, `ea_trax` to `UiStyle::Original`), is deprecated, and nothing writes it.
2. **A new engine-generic crate, `libs/blackbox-ui`** (Bevy-free, no NFS:MW names). It holds:
   - the neutral draw tree (`UiTree`, `UiNode`, `NodeKind`), moved out of `libs/blackbox-feng/src/tree/mod.rs`
     and re-exported from `blackbox-feng` so existing paths still compile;
   - the `Component` trait (`layout`, `build`, `handle`), `DrawList` and small geometry and input types;
   - the base components (`Panel`, `Stack`, `Text`, `Image`, `Gauge`, `Table`, `Row`, `Switch`);
   - the slot machinery (`ElementId` and its registry), the `Theme` and `Themes` traits and the `resolve` fallback.
3. **One presenter, still.** Both producers of the tree are allowed: FEng (for Original) and the component
   system (for Custom and later themes). Both emit `UiNode`s. `ui/present/blackbox.rs` does not change. This
   adds a second *producer* of the neutral tree, not a second *draw path*. The egui radio card is retired, which
   returns the HUD to a single pipeline.
4. **Overridable slots with an Original floor.** A stable catalogue of element slots (`ElementId`) covers the HUD
   and the front end. Every slot has an Original implementation. A theme may return a component for any slot or
   return nothing. `Themes::resolve` asks the active theme first and falls back to Original on `None`. A theme
   therefore needs to implement only what it has converted. `Custom` implements `RadioCard` and returns nothing
   else, and still renders a complete UI.
5. **Original delegates; it does not re-author.** The Original components are adapters over the code that exists
   today: HUD adapters wrap `HudBinding`, `minimap`, `pursuit_meter`, `blips` and the gauges; menu adapters wrap
   `ScreenLogic`, `WidgetMenu`, `IconMenu` and `options_rows`. Original is pixel-identical by construction.
6. **`Remastered` is an enum variant and an extension point, not a theme.** Until a Remastered theme exists, every
   slot falls back to Original. Section 10 of the spec gives a worked design note (a fullscreen map with a legend
   on the right) to show that the component set is enough for it.
7. **A `bb.ui` mod hook** (mirroring `bb.menus` and `bb.hud`, see [ADR 0005](0005-native-mod-plugins.md)). A mod
   may register a theme or override individual slots. A registered mod theme becomes a selectable style. How a
   mod passes a component across the C ABI is not decided here; see "Open questions" below.
8. **Reuse existing shapes; invent no new idiom.** The design copies three in-repo patterns:
   - the `BlipStyle` trait (`crates/nfsmw/src/hud/blip_style.rs`): a runtime-registered, swappable HUD widget
     with a `name()` key and default methods;
   - the mod `InterfaceRegistry` (`libs/blackbox-modding/src/host/registry.rs`): `add_*` ports keyed by id;
   - the swappable-renderer interface of [ADR 0004](0004-swappable-renderers.md): one trait, several
     implementations, one selected at start-up, with a fallback when the choice cannot run.

## Why not the alternatives

- **A light skin provider only** (colours and fonts, no structure): cannot express a different layout, such as a
  fullscreen map with a legend. The owner asked for a full component framework.
- **Each theme as a separate egui UI:** a second presenter. It breaks ADR 0002 ("exactly one presenter") and
  repeats the two-pipeline problem that the radio already has.
- **Reuse `VisualStyle` for the UI:** graphics and UI looks are chosen independently. The owner asked for a
  dedicated switch.
- **Build the Remastered theme now:** speculative. Its parts are listed as the extension point. The fallback
  floor lets it arrive later without changing the framework.
- **Bevy UI nodes as the component system:** Bevy UI has no equivalent of FEng scripts and messages, so the
  runtime would still be needed (ADR 0002, "Bevy UI now"). `libs/` must also stay Bevy-free. The neutral tree
  keeps a Bevy UI presenter possible later without touching the components.
- **An ECS for UI entities:** the game is not an ECS (ADR 0005). The UI follows the same fixed-frame model as
  the rest of the game.
- **Per-screen theme choice:** more settings, more combinations to test, and no fallback story. One switch with
  one floor is simpler.

## Consequences

- **Costs.** The tree moves between crates (mechanical, but many imports change). The Original adapters are a
  new layer over existing code and must stay exactly as they are today. The slot catalogue and the resolver are
  new surface that must track the HUD and front-end code.
- **Single presenter restored.** Retiring the egui radio path removes the only UI that bypasses the presenter.
  Metrics and console overlays are egui and stay so; the spec lists what must still work without the old branch.
- **`Remastered` will look like Original** until a theme implements it. The settings row can show it, but the
  owner should decide whether to ship it visible (see "Open questions").
- **Scope discipline.** Each slot and each base component needs a consumer. Empty interfaces are not added.
- **Verification.** Resolver fallback unit tests in `blackbox-ui`; base-component layout and build tests; `UiStyle`
  parse and merge round trips; mod registration tests modelled on `crates/nfsmw/src/modding/backends/menus_tests.rs`;
  the existing HUD and front-end suites; offscreen `--screenshot` runs under `--ui-style original` must match
  the pre-change output (the 256 by 144 digest tests in `libs/blackbox-render/src/gpu/parity/digests/`).

## Open questions (not decided here; for the owner or the parent task)

1. **Where `ElementId` lives.** The plan's sketch lists MW names (`Tachometer`, `PursuitMeter`, `MainMenu`, ...)
   inside the enum and also says the concrete set belongs in `crates/nfsmw`. [AGENTS.md](../../AGENTS.md) forbids
   NFS:MW names in `libs/`. This ADR proposes that `blackbox-ui` owns a generic slot id, and that `crates/nfsmw`
   owns the MW enum and its mapping. The spec records this as the working assumption.
2. **Mutable state in `build`.** The sketch has `build(&self, out)`. The FEng runtime advances scripts and
   messages, which needs `&mut`. The spec proposes that state advances in `layout` or in a separate update call.
   The trait signature is not final.
3. **Mod themes in `UiStyle`.** The plan names a `UiStyle::Mod(id)` variant. The settings pattern (`names!`, a
   fixed `ALL` array, cycling in the options row) assumes a closed enum. A dynamic variant needs a different
   cycle and a different serialised form.
4. **Component values across the C ABI.** ADR 0005 allows only `#[repr(C)]` data and `extern "C"` functions across
   the boundary. A `Box<dyn Component>` cannot cross. A mod would have to describe its component tree through
   host calls that return handles, and the host would wrap them in `Component`. This is the largest open design
   point of the mod hook.
5. **Live switch or restart.** Answered (T10, T12): the theme is resolved every frame, so a change takes effect on the
   next frame with no restart.
6. **Default.** The plan sets `UiStyle` to `Original`. `VisualStyle` defaults to `Custom`
   (`crates/nfsmw/src/settings/appearance.rs`). The different default is intended, but the owner should confirm it.
7. **Milestone number.** The plan does not give this work a milestone number. The header lists the milestones it
   touches.

## Outcome (T12)

- The HUD and the menus draw through the composer (`ui/theme/compose.rs`, [ui-themes-compose.md](../specs/ui-themes-compose.md)).
  The presenter is unchanged, and the egui radio path is gone. Under Original the composed trees equal the trees the
  plugins store, node for node and in draw order, except that a slot root keeps no parent link. Synthetic tests prove this.
- Owed, each on the machine that can run it: the 256 by 144 digest parity and the `--ui-style custom` screenshot (a GPU),
  the real-package "no node claimed twice" check (game data), the Mods-page selector row, a `Gauge` primitive for mods,
  and the decision whether a mod's HUD follows the viewport.

## Could not verify

- Whether the FEng packages for every menu screen can be wrapped one by one without changing their scripts. T8
  must check this against each screen.
- The exact Original source for `Career` screens and `TrackSelect`. The mapping in the spec is marked where it
  is not yet confirmed in the code.
