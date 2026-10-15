# Common sound events

- Date: 2026-10-14.
- Specification: [common-audio-events.md](../specs/common-audio-events.md).
- Implementation: game audio `common_effects`.
- Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0-1.0, `SFX_Common`, class definitions and main-state sound enums.
- Independent PC checks: message constructor selects class IDs0/2/1 and
  initializes remaining words0; complete process-update body reads packed
  mixer volume slots2/3/4 and clears message inputs afterward.
- Original process-update replay with independent packed mixer words confirms
  these three slot assignments and clearing all five message inputs. CSIS
  reference-count and commit callbacks are declared stubs for this producer
  check; they are not original graph or PCM validation.
- Specification written before implementation. No restricted code or
  extracted bank data is included.
- Limits: original sound-clock dilation and full DSP/PCM equality remain
  unverified; authored module routing does not establish these behaviors.
- The normal-play delivery regression is based on remake source inspection:
  its pause wrapper omitted the common-event forwarding method and inherited
  the trait's empty queue. This is a host integration correction, not a new
  original sound or a change to the authored gain, bank, IDs or mixer slots.
