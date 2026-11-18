# Front-end audio implementation provenance

- Specification: [frontend-audio](../specs/frontend-audio.md).
- Date: 2026-10-23.
- Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0; source names listed in the specification. Read for behavior only.
- PC 1.3 clean executable SHA256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Ghidra identity checked before reading menu sound dispatch. PC inspection
  corrects the incomplete reference's SOUND_UP mapping: trigger 0, not 103.
- Implementation: `audio/menu.rs`, `audio/plugin.rs`, and frontend sound dispatch.
  Uses the existing AEMS interpreter and runtime installed banks; no audio ships.
- Regression: semantic messages, ignored releases and queue consumption; ignored
  installed test renders every supported trigger, checks audible slots, preserves
  authored silent slots and checks natural completion. Execution results belong to the local validation report.
- Limits: no matched listening comparison, original FEHUD bus gain, sustained
  rap-sheet sound or complete screen-specific sound table parity is claimed.

- Follow-up installed test diagnosis: common trigger 6 and front-end triggers 28
  and 88 select authored zero sample entries. Static graph wiring copies the
  class ID directly to player selection. Both inspected PC bank sets agree;
  silence is preserved instead of inventing replacement audio. The regression
  verifies these references and aggregates playback/lifetime outcomes at 30,
  60 and 144 FPS. Source tests are authored; execution outcomes belong to the
  current local validation report.

## Pursuit results, race results, milestones and car selection

- Date: 2026-11-18. The PC 1.3 executable above was identified through Ghidra.
  The install used for package and bank inspection carries the same executable hash.
- **[exe]** `0x00516560` and `0x00585700` route FEng sound posts to the screen of
  the originating package. `0x005724D0` maps the hash and calls the screen's
  virtual sound filter at slot 2. It calls `0x004AE8F0` only if the filter returns a trigger.
- **[exe]** The primary pursuit, milestones, and car select tables are at
  `0x008A1730`, `0x0089EEDC`, and `0x008B7E3C`. The respective filters
  `0x00573C00`, `0x00511900`, and `0x007A3B20` preserve the generic result for
  the listed cues, including the absent trigger for `4ADDE92C`.
- **[exe]** The results table at `0x008A1708` selects `0x0056A3E0`. The
  results constructor `0x005A6EC0` initializes the racer count and page. The
  sound override checks both fields and tests the overlay script through `0x00524A10`.
- **[install]** The package scripts and responses establish the send sites. The
  installed module banks establish the referenced sample indexes. The Rust
  regression checks these slots with the offline mixer and correlates each
  routed message with its trace trigger and voice.
- Trigger names come from the `eMenuSoundTriggers` enumeration in
  `AemsDef.hpp`. `0x007A34F0` starts the impound-state animation. The car
  selector's handler at `0x007C1770` posts the Showcase transition.
- Implementation: `audio/menu.rs` and the results screen sound filter.
  Frame deduplication keeps every pursuit tally cue. No matched original
  runtime sound trace or listening comparison was performed.

## Complete dispatch and sound-state follow-up

- PC 1.3 dispatch at `0x005724D0` supplies the complete generic table.
  The earlier package-owner and virtual-filter reads apply to all 142 entries.
- Screen conditions were checked against `0x005108E0`, `0x00511540`,
  `0x005116F0`, `0x00511800`, `0x00511900`, `0x00511960`, `0x00511E70`,
  `0x00511FA0`, `0x00519450`, `0x00573810`, `0x00573C00`, `0x00573C60`,
  `0x0057DCE0`, `0x0056C660`, `0x007A3B20`, `0x007A6ED0`, and
  `0x007A7130`. The corresponding named classes in the local reference
  identify fields and modes. `0x005A6EC0` selects page 2 for race type 4.
- `0x005116D0` and `0x0055A760` establish the race sheet's activated-event
  lifetime. `0x005529F0` and `0x005881C0` show that normal bounty rows are
  created unlocked after the stage filter.
- `0x007C1770` shows the purchase dialog and receives its purchase
  callbacks. Installed `Dialog.fng` provides the answer-button sound
  response. The dialog plays its ordinary accept cue for the purchase.
- `0x004B9470` supplies the rap-sheet range and replacement rule.
  Login notification `0x0052C670` stops a matching held cue through
  `0x004AE940` and `0x004AF5D0` when the player skips either animation.
  `0x004B9900`, `0x004AECB0` and `0x004EB7D0` supply the common gain
  and preview handling. `0x004B59F0` supplies the master-volume controller.
  `MAPOUTPUT.mxb` verifies the base, slot offsets and connections for the two
  FEHUD outputs.
- `0x004AF1E0` is a return instruction. `0x004B9080` and the action
  table at `0x008EE570` implement the effective state changes.
  `0x004B5C70` publishes their mask to the mixer. Photo construction,
  shutter and destruction are at `0x0058E630`, `0x005A7EA0` and
  `0x0057D880`. The boot movie constructor and notification handler are at
  `0x0058ECA0` and `0x0057DDA0`. Movie completion at `0x00542C50`
  clears the player's Pause state. The installed mixer graph connects bits
  3–5 to world attenuation and bit 6 to FEHUD suppression.
  Stream initialization at `0x004C7D20` identifies the first three actions
  as Pathfinder music, speech and NIS/SFX pause ownership.
  The generic in-game constructor at `0x0058F180` has no direct FMV state
  call. Its caller-dependent pause ownership still requires comparison.
