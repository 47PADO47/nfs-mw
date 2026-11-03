# Continuous UI reference recording

The Windows recorder records one selected executable's window, speaker audio,
foreground keyboard/pointer state and XInput changes. Captures remain outside
the source checkout. It does not alter installed game files or send input.

## Start and stop

Use Python 3.13 with `PyAudioWPatch==0.2.12.9`, and FFmpeg/FFprobe with
`gfxcapture` and working `h264_nvenc` support. The original-game instrumentation
is a separately supplied private probe. This repository contains no executable
addresses, probe profiles, original game data or translated game code.

```powershell
python -m tools.ui_capture.session --exe '<absolute executable path>' --output '<private capture directory>'
```

The selected executable must have exactly one visible window. The session panel
shows elapsed time, recorded frames, sample duration and marker count.

- Ctrl+Alt+F9: add a marker without leaving the game.
- Ctrl+Alt+F11: stop and finalize.
- The panel also has marker and stop buttons. Closing it stops safely.
- `--seconds 15 --headless` performs a short bounded test without a panel.

Keep the game visible and rendering. Minimized or inactive games can stop
delivering frames. The recorder stops on game-window closure, encoder failure,
audio queue overflow or the disk reserve. It preserves partial output.
Three GiB remain reserved by default; `--reserve-gib` changes that threshold.

Audio is **all applications on the selected speaker output**, not per-game
audio. No microphone is opened. `--audio-device` selects a WASAPI loopback device
index. Switching the default device during recording does not switch this
stream; restart the session after changing outputs. System sound/music can
mask menu effects; capture evidence must identify the actual mix/settings.
WASAPI can omit an interval without setting an overflow flag. ADC timestamp
jumps above 25 ms insert a bounded silence interval to preserve later timing.
`audio_gap` marks that interval as unobserved content, and audio evidence is
reported incomplete even though the media remains playable. No missing sound
is reconstructed. Smaller gaps/drift remain visible in the retained clocks.

## Session files

`session.json` records source state, installed executable/mod/UI-asset hashes,
clock anchors, commands, stream identities, skipped probes, loss and outcomes.
`events.jsonl` records inputs, markers and health on a monotonic timeline.
`og-ui.jsonl`, when a private probe is supplied with `--probe`, retains its
independent original-game event timestamps and observation limits.

`video.mkv` and `audio.flac` are independent durable originals. `recording.mkv`
combines native-size video and lossless sound with chapter markers. `review.mp4`
copies the video and encodes an AAC viewing track. `review.html` provides a
searchable event index, including package changes, focus, sound requests and
animation starts when the private probe observes them. Original tracks remain.
One recording can cover many menus; markers and package events locate them
without recording each screen again. The index starts a quarter-second before
a selected event. Unknown/unvisited cases remain unknown.

Rebuild the review or inspect actual media:

```powershell
python -m tools.ui_capture.review '<session directory>'
python -m tools.ui_capture.quality '<session directory>'
```

## What timing establishes

Audio callback sample indices and PortAudio ADC/current-time pairs are retained
against the monotonic clock. Video packets use frame receipt wall time relative
to the session anchor. Periodic clock checks expose wall/monotonic drift.
WGC, drivers and encoding add uncalibrated latency. Matching durations do not
establish frame-accurate A/V alignment. Use a known visible/audible transition
for calibration before making fine timing claims.

Capture requests at most 60 frames/second and preserves actual variable frame
timestamps. It does not synthesize intermediate frames. A nominal 60 FPS stream
does not prove 60 unique captured frames each second. `quality.json` reports
the measured intervals and decode checks. Inspect frames for black/frozen
output as well as checking nonzero sound. No automatic parity pass is implied.

## Reference and remake comparison

Use the chosen modded PC installation as the observed reference, including its
controller support. Record different icons and extra menu options as explicit
presentation/feature allowances. Keep those separate from behavior mismatches.
The remake's [opt-in UI trace](ui-parity-tracing.md) supplies its own host events
for matched scenarios. Scripted action replay does not prove physical input.

For a disabled action or unexpected popup, bookmark the recording and record
the state, attempted action and resulting event. Consult existing behavioral
specifications, then targeted PC/Xbox research if the reason remains unknown.
Xbox generated code is static evidence from a different platform, not proof
that the modded PC process uses the same condition. Record platform, revision,
function reference, inferred condition and confirming/contradicting PC capture.
Keep binary research private and write an independent spec/provenance before
changing public behavior. This recorder does not automatically decode reasons.

## Upstream interfaces

[FFmpeg gfxcapture](https://ffmpeg.org/ffmpeg-filters.html#gfxcapture) supplies
window capture; [PyAudioWPatch](https://github.com/s0d3s/PyAudioWPatch) supplies
WASAPI loopback. Both run as external dependencies; their source is not copied.
