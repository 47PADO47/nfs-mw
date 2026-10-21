# Story mode plan

Approved 2026-10-21. Order: EAGL animation decoding, then the NIS (in-engine cutscene) player, then the story flow. Specs and provenance follow [licensing.md](../licensing.md) (spec first): reader agents turn the CC0 decomp and Ghidra findings into prose specs; separate implementer agents write the Rust from those specs only.

## M0: Setup

- M0.1 SUP: create the worktree and branch, install the hooks, get a green baseline on all four guard rails.
- M0.2 RDR: spec skeletons and index rows.
  - Animation: `eagl-animation.md` (restructured), `eagl-animation-bank.md`, `eagl-animation-codecs.md`,
    `eagl-animation-channels.md`.
  - NIS and story: `nis-scenes.md`, `nis-player.md`, `nis-events.md`, `subtitles.md`, `story-flow.md`.
  - Provenance: `eagl-animation.md`, `nis-player.md`, `story-flow.md`.
- M0.3 SUP: a condensed copy of this plan in `docs/plans/story-mode.md`.

## M1: EAGL animation (new crate `libs/blackbox-anim`, Bevy-free)

**Specs (parallel):**
- S1: ELF object loading, skeleton, bone record, bind pose.
- S2: bank, animation record, type ids, channel-to-bone binding, frame rate. Retract the old type guess.
- S3: delta codecs (DeltaQ/QFast/SingleQ/F1/F3, the 14-byte key, nibble and byte deltas, interpolation).
- S4: stateless, raw, compound, phase, CSIS event and pose channels.
- G1 (SUP, after the census): Ghidra for the 0-byte codecs that the install actually uses, with addresses from the
  decomp's `symbols.json`. Rename the old `FUN_*` table.
- G2 (SUP): a worked example of one key decode, by hand, written into the spec.

**Implementation (IMP, one commit each):**
- I1: crate scaffold, workspace member, a row in `docs/architecture.md`.
- I2: `elf.rs`, the ELF32 `ET_REL` reader (strip the `0x11` pad, apply `R_MIPS_32`), plus a test ELF builder.
- I3: `skeleton.rs` (bones, parents, world pose).
- I4: `bank.rs` and `type_id.rs` (unknown types become `Unsupported(id)`).
- I5: `crates/nfsmw-data/src/anim.rs` (chunk finder) and the CLI `dump-anim <file> [--bank] [--sample N]`.
  - SUP then runs a **census** of codec types over `NIS/*.bun`, `L2RA.BUN` and `InGameB.bun`.
  - The counts go into `formats/animation.md` as [verified]. They set the codec order and which G1 gaps matter.
- I6: `codec/delta.rs`, the shared delta core.
- I7–I13: one file per codec under `codec/`, behind a `Channel { frames, sample }` trait, implemented in census order.
- I14: compound and event channels.
- I15: `pose.rs` and `eval.rs` (`Animation::sample(t) -> Pose`).
- I16: real-install test `real_install/animation.rs`:
  - all 188 objects, 64 skeletons, 2,532 bones and 124 banks parse;
  - every frame decodes;
  - |q| = 1 ± 1e-3;
  - the 99th percentile of the per-frame angular step is under 0.2 rad;
  - the airliner `_q` channel moves smoothly.
- I17: `scenes/anim_view` plus CLI `view-anim <Scene|file>`, a skeleton stick viewer with no renderer change.
- I18: the `view-assets` hook plays the `L2RA.BUN` world animations on rigid parts (the airliner).

**Gate:** the crate tests and I16 pass, `dump-anim --sample` shows continuous quaternions, and SUP's screenshots show
`view-anim ArrestF02` and the airliner moving.

**Fallback:** a channel that cannot be decoded holds its first key. The I16 test lists those channels by name.

## M2: NIS player (new crate `libs/blackbox-nis` + `crates/nfsmw/src/scenes/world/nis/`)

**Specs.** These can start during M1 implementation.
- S5: `AnimScene` header and entity (152 B each), `AnimDirectory`, life cycle, start and vanish frames, car binding,
  camera-track choice.
- S6: the `ENIS*` event parameters and the subset the 59 CARP sequences use.
- S7: NIS audio.
- G3 (SUP): Ghidra for:
  - the NIS activity, completion and skip;
  - the camera track per scene;
  - the audio stream choice;
  - the letterbox;
  - freeze rules;
  - the arrest spot;
  - movies with no file replaced by `Scene_FMV*` with the FMV camera pack.
- G4 (SUP): the 60-byte skinned vertex (blend indices, bind pose).

**Implementation:**
- N1: scene chunk parser.
- N2: scene timeline.
- N3: `nfsmw-data/src/nis/bundle.rs`, the bundle loader.
- N4: shared `CameraLibrary { nis, fmv }`. This refactors `nfsmw-data/src/world/moment.rs` and does not need M1.
- N5: directory data.
- N6: `NisPlayer` on `WorldScene`.
  - Move `Anchor` out of `scenes/world/moment.rs` into `scene_space.rs`.
  - ICE camera override, letterbox, console `nis list|play|stop|status`.
- N7: car actors (entity car N maps to a world car, with physics frozen).
  - Check: in IntroNisDD the end position of car 0 is within 5 m of the 16.1.0 grid spawn.
- N8: props.
- N9: read the skin weights in `libs/blackbox-solid` (tell it apart from `WorldNormalMap` by effect id).
- N10: CPU skinning in `blackbox-anim/src/skin.rs`, with a `rigid_split` fallback.
- N11a–c: `update_mesh_vertices` on `RenderBackend`: the trait plus the mock, then wgpu, then Bevy.
- N12: skinned characters.
- N13: CARP `ENIS*` events: Freeze, HideCharacter, CopLights, ScreenFlash, OverlayMessage, CarShake, Lights. Any
  other event is logged once.
- N14: `audio/nis.rs`, reusing the `SpeechIndex` path of the speedbreaker moment.
- N15: skip and `NisOutcome`.
- N16: first consumer: being busted plays the arrest NIS in `busted/flow.rs`. The current hold stays as the fallback.

**Gate:**
- `nis play ArrestF02` shows the camera cuts, the animated cars, the skinned characters and the sound, and Accept
  skips it.
- Being busted plays the arrest NIS.
- A real-install test builds all 87 bundles.

**Fallbacks:** rigid split, or hidden characters, when skinning fails; `Track 0` when the camera track is unknown;
silence when the audio stream is unknown; the player's pose as the anchor.

## M3: Story flow (`nfsmw-career/src/story/` for the rules, `nfsmw` for the wiring)

**Specs:**
- S8: subtitles (the decomp's `SubTitle.cpp`).
- S9: `story-flow.md`, every story state machine in implementable form, built from the existing specs.
- G5 (SUP): Ghidra for:
  - the bit that marks a node completed;
  - the forced-car attribute;
  - what `IntroMessageID` shows;
  - the order inside `DoSafeHouseIntro` and the point where the stage goes 16→15;
  - the reward marker-kind table at `0x0062AA90`;
  - the final race's trigger names;
  - the `GameWon` chain;
  - the `JumpToCarLot` context bits.

**Implementation.** These start early, during M2: T1, T2, T3, T7, T18, T21, T23, T25.

Data and movies:
- T1: `gameplay/story.rs`, `RaceStory`: intro and outro NIS and movies, intro message, `QuickRaceNIS`, forced car,
  rewards.
- T2: `blackbox-text/src/subtitles.rs`.
- T3: extract `movie/playback.rs`.
- T4: subtitle overlay.
- T5: in-world story movie with the simulation frozen (`WorldScene::fullscreen` and `take_clip`), console `movie`.
- T6: `frontend/flow_movie.rs`, which generalises `Stage::Tutorial` into `Stage::Movie { then }`.

Race life cycle:
- T7: intro and outro step rules, following `career-scripts-events.md` §1.1.
- T8: `race/intro.rs` (`Stage::Intro`).
- T9: `race/outro.rs` (`Stage::Outro`).
- T10: the NIS shell race 16.8.1.
- T11: the forced preset car `M3GTRCAREERSTART`.

Prologue:
- T12: prologue rules and resume.
- T13: `flow_story.rs`: a new career starts 16.1.0.
- T14: the scripted 16.2.1 chase: sabotage at 15 s or at `kill_engine`, call 10, 2 cops, bail, fade.
- T15: JumpToCarLot.
- T16: `drive_to_target`: SMS 15, SMS 121 after 120 s, GPS.
- T17: DoSafeHouseIntro: `storyfmv_saf25`, then the rival bio, then stage 16→15. `enter_first_bin` moves from the
  engage to the bio.

Phone:
- T18: SMS delivery opens `SMS_Message.fng` with the game paused. The placeholder card goes.
- T19: cell-call voice through the speech `cellcall` event with `VoiceTake`.
- T20: inbox delete and replay.
- T21: `settings/nodes.rs`, a decoded view of the node states.

Rivals:
- T22: the Challenge rival button runs the gauntlet (IntroNisBLxx through T8).
- T23: gauntlet rewards: pink slip into the stable, cash bonus, out-of-jail-free, impound release and box, parts.
- T24: `frontend/flow_arrival.rs` drives `BlacklistStepper` (bio, then the `blacklist_NN` movie, then the rap sheet
  movie and the overview).

Final race and resume:
- T25: final race rules: call 95 at 10 s; call 96 after 300 s within 500 m; door; win at ≥ 180 km/h with a direction
  dot ≥ 0.8.
- T26: lift the `epic_pursuit` refusal and map it to `1.8.1`; triggers, door, GPS.
- T27: `flow_endgame.rs`: EndingNis04, then `CAREER_COMPLETE`, then `GameWon.fng`, then the `EndGame` stepper.
- T28: resume rules: 1.2.3 completed and the career not complete forces 1.8.1; a pending prologue event is
  re-engaged.

Tools:
- T29: if G3 confirms it, a movie with no file is replaced by `Scene_FMV*`.
- T30: debug console `story status | goto prologue <n> | goto arrival <bin> | goto final`.
- T31 (optional): mod events `NIS_STARTED`/`FINISHED` and `MOVIE_*`.

**Gate** (offscreen `--ui-script` runs with T30):
- A new career plays rac01, then IntroNisDD, then race 16.1.0.
- 16.2.1 is followed by the car lot, the drive, saf25 and the bio at stage 15.
- A bin-15 challenge plays IntroNisBL, both races, the rewards, then the arrival sequence and `blacklist_15`.
- `story goto final` reaches the win, then EndingNis04, then `GameWon.fng`.

## M4: Docs and PR

- D1–D3 IMP: `docs/roadmap.md` gets a story section and drops the "no NIS player" blocker. Fix the stale
  `provenance/career-runtime.md:67`. Rows for `blackbox-anim` and `blackbox-nis` in `docs/architecture.md`. Fill the
  provenance "Implemented / Checked by" fields.
- D4 SUP: full guard rails, the whole real-install suite, and the visual checks of every gate.
- D5 SUP: push and open the PR with `.github/PULL_REQUEST_TEMPLATE.md`, every item addressed. Bind it with the
  ccd_pr tools.


## Key risks

| Risk | Fallback |
|---|---|
| A codec exists only as a 0-byte `.cpp` | The census limits the Ghidra work to codecs that are used; the rest hold their first key. |
| The bank layout disagrees with the old inference | Rewrite the spec from the decomp headers plus the G2 worked example. |
| Skin index meaning, renderer change in two backends | `rigid_split` (no renderer change), or hide the characters. |
| Scene anchor convention | The grid-spawn check in N7; fall back to the player's pose. |
| No Lua VM: the story is built from prose specs | S9 makes each graph explicit, with tests per state; the T30 console helps find divergences. |
| Files near 500 lines (`race/setup.rs` 495, `frontend/plugin.rs` 486, `flow.rs` 467) | Every task puts its code in new files. |
| Decomp code leaking into the repo | Reader and implementer agents are kept apart, SUP reviews every spec, and the leak check runs. |

## Verification (end to end)

1. The four guard rails, workspace-wide.
2. `cargo test --release -p nfsmw-data --test real_install -- --ignored`: animation, NIS bundles, story data,
   subtitles.
3. Offscreen screenshots: `view-anim ArrestF02`, the airliner, `nis play IntroNis05` / `ArrestF02` in
   `view-world --drive`, being busted.
4. The four story `--ui-script` runs in the M3 gate.
5. SUP spot-checks of the audio and subtitles in `play-movie blacklist_01` and a story NIS.
