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
