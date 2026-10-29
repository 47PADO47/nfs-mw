# Frontend capture telemetry

`NFSMW_UI_TRACE` enables observation of the remake's existing frontend host.
Set it to an absolute path for a **new** JSONL file in an existing directory
outside the source checkout, then launch the game normally. An existing file is
never overwritten. An invalid destination logs an error and leaves tracing off.
Without the variable, the host does not collect samples, allocate records or
start a writer thread. This does not change menu behavior or select a UI theme.

## Records and clocks

Every record has `kind`, ordered `seq`, `elapsed_ns` and `data`. Normal records
also have `frame`, the count of frontend screen updates since the trace started.
`seq` and elapsed time start when the screen host is constructed, not at the
first video frame. The header records Unix nanoseconds as a decimal string,
process ID, schema `nfsmw-ui-trace-v1`, sample period and coverage. Use this clock
anchor to align an external recorder; it is not sample-accurate audio alignment.
The frontend update count is not a renderer presentation count.
Host events also carry `data.ui_seconds` (sum of positive finite frontend steps)
and `data.frame_dt`. Scripted UI time can advance faster than wall time; preserve
both clocks. Device and stack events between updates carry the last update's
counter and step, with their own actual monotonic timestamp.

| Kind | Observation |
| --- | --- |
| `input` | `dt`, incoming host action mask, resolved menu pad mask and neutral-input gate; on change and a one-second heartbeat |
| `device` | Prompt device, presence of an active controller, disconnect flag and scripted-input flag; on change |
| `stack` | Package IDs/names after each open, pop or clear, in bottom-to-top order |
| `game_message` | Runtime message ID, package ID and optional sender GUID, before host dispatch |
| `sound_request` | Runtime sound message before batching/deduplication; **not proof of mixer dispatch or audible playback** |
| `package_request` | Runtime package command, target name and originating package ID |
| `state_sample` | Package focus, available logical list selection/enabled flags, focused-object and root-object state |
| `end` | Normal shutdown and loss count since the previous record |

Local object samples contain GUID/name hash, parent GUID, script ID/clock,
hidden flag, local alpha, position, size and rotation quaternion. They exclude
resolved strings, glyphs, textures and asset bytes. At most 32 objects of the
top package are sampled: its focus first, then root objects in package order.
`objects_truncated` reports whether this selection exceeded the cap. Child
objects apart from the focus are outside the sample, even when that flag is
false. Samples run at most once per 33 ms of wall time and unchanged samples
are omitted. They are **local state before presentation**, not inherited world
transforms, final visibility, replacement prompts or rendered pixel evidence.

## Long captures and limitations

The file writer uses a bounded 256-record queue and never waits for disk writes
on the gameplay path. `seq` gaps and `dropped_since_previous` expose overflow;
a comparison must treat an affected interval as incomplete. Shutdown drains
the queue and flushes the file. A crash may lose buffered tail records; a
missing `end` is an incomplete trace, not a successful stop. Disk errors are
logged and tracing stops without terminating gameplay.

The trace records outgoing runtime messages, not every internal FEng message
or script start. State samples can miss short scripts between samples. An
unavailable logical list produces `null` selection/enabled fields; package
focus alone does not establish the screen's logical selection. Direct host
sound requests outside the outgoing runtime batch are outside this trace.

The incoming host pad mask is already mapped from actions. It cannot establish
physical button mapping, raw analog values, pointer events or typed text. The
device event reports the host's result, not every raw device event or controller
identity. Capture those with an external input
recorder. Use synchronized video and audio to verify visible animation and
actual menu sound. Scripted `--ui-script` runs remain useful for deterministic
checks, but do not prove physical input behavior.

These diagnostics observe independently authored remake code. They contain no
original executable addresses or implementation copied from binary research.
Record source revision, executable hash, settings, reference identity and
recorder clock anchors in the external session manifest. Traces and reference
media stay outside the publishable checkout.
