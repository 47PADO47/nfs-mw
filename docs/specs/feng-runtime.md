# FEng runtime (scripts, messages, drawing)

How NFS: Most Wanted runs a loaded FEng package: object state, animation scripts, messages and responses,
buttons, and the rules that turn objects into a draw list. The on-disk layouts are in
[formats/frontend.md](../formats/frontend.md); fonts and text are there and in [formats/text.md](../formats/text.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/`: `FEng/{FEngine,FEPackage,FEObject,FEScript,FEKeyTrack,FEKeyInterp*,FEMessageResponse,
  FEMsgTargetList,FEEvent,FEGroup,FEString,FEMultiImage,FEMath,FEPackageReader}.{cpp,h}`,
  `Frontend/{FEngRender,FERenderObject,FEngFont,RealFontOld}.{cpp,hpp}`, `FEngInterfaces/FEGameInterface.cpp`,
  `HUD/*`, `Localization/*`. Read for understanding; no code copied. The tick rate and the draw order were
  checked against the install by reading the packages (see [feng provenance](../provenance/feng.md)).
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). Everything here is **[decomp]** unless
  marked; nothing is checked against the running game.

## 1. Screen and time

- The logical screen is **640 × 480, origin at the centre, +y down**; the view matrix is the identity. Wide
  and thin screen variants are separate packages (`WIDESCREEN_GLOBAL.BUN`), selected by the game.
- Time is counted in **ticks**. A script length, a key time and an event time are ticks. The game passes the
  frame's ticks to `FEngine::Update`; scripts in the HUD suggest **16 ticks per frame at 60 frames per
  second, so 960 ticks per second** (`FADEIN` is 600 ticks, 0.625 s) **[inferred]**. The runtime takes a time
  step in seconds and converts.
- A package is updated once per frame, objects in list order, children of a group after the group itself.
- **Presentation initialization [ours]:** loading a package evaluates its `INIT` scripts with zero elapsed
  ticks before returning a drawable tree, including their immediate chain targets. This prevents a newly
  switched screen from briefly showing its stored end pose before its entrance animation. Other packages,
  pad input and the global clock are unchanged; queued initialization events are processed on the next update.

## 2. Object state

Each object keeps the block `SA` as live state: colour (blue, green, red, alpha), pivot, position, rotation
(quaternion), size, and for images the UV rectangle. Scripts overwrite these; the host may overwrite them
too (rotation of a needle, alpha of a warning). Besides the block an object has:

- identity: GUID, name hash, parent GUID (0 = top level), resource index, type;
- `Flags` (game bits): bit 1 = text not localized, bit 3 = not drawn on the PC. Objects are hidden by alpha 0
  (the `HIDE` script) or by the host; everything below a hidden group is hidden too;
- a **current script** (initially `INIT`, time 0) and its clock;
- strings: the text (UTF-16), the label hash, the justification, leading, maximum width and the font;
- images: the texture handle (resource hash) and the UV rectangle; multi images: three texture hashes.

Objects are found by **`FEHashUpper(name)`** = `bStringHash(UPPER(name))` (the game's lookup key) or by GUID.
Several objects can share a name hash; a lookup returns the first.

## 3. Updating scripts

Per object per frame, with `dt` ticks (`bExecuting` is true while the package runs):

```
prev = script.time ; script.time = max(prev + dt, 0)
if script.time >= script.length:
    if script chains to another script:        # Sc
        apply tracks at the old script's end ; fire events (prev, length]
        switch to the chained script ; its time = script.time - length ; fire events [0, time)
    else by the end behaviour (flags & 3):
        0 once      : fire events (prev, length] ; time = length + 1       # the last key stays
        1 loop      : fire events (prev, length], then time = time mod length, fire [0, time) ; re-arm move-to tracks
        2 ping-pong : time = time mod (2 * length)
else: fire events in (prev, time] (looping scripts wrap, ping-pong plays backwards after the turn)
apply the tracks at script.time (skipped once a "once" script sits at length + 1 and nothing changed)
update the children of a group
```

An event fires when its time lies in the half-open interval just advanced; at the end of a script the
interval includes the last tick. **Tracks** are evaluated in order: the first track (output offset 0, the
colour) always; **the other tracks only while the object's alpha is not 0** (an object faded out stops
animating its geometry).

**Interpolation** (`track.interp & 0x7F` is the end behaviour of the track, the same 0/1/2 meaning; the
interpolation type is in the track header):

- type 0 **none**: the value of the key at or before the time (steps);
- type 1 **linear**, type 3 **move-to**: let `key` be the first delta key with `time >= t` and `prev` the
  key before it. If there is no `prev`, or `t >= key.time`, the value is `base + key`. Otherwise
  `base + prev + (key - prev) * f` with `f = (t - prev.time) / (key.time - prev.time)`. Integer and colour
  values are rounded (`+ 0.5`, truncate); vectors add per component; **quaternions**: `base * slerp(prev,
  key, f)` (normalised lerp when the dot product exceeds 0.999, the shorter arc otherwise);
- types 2 and 4 are not implemented by the game (the value is left alone);
- **move-to** tracks, when their script starts (set by a response, by chaining or by a loop restart), rewrite
  the first delta key to `current value - base`, so the motion begins where the object is;
- a track with no delta keys writes its base value.

A track writes into the object state at its offset (colour 0, pivot 4, position 7, rotation 0xA, size 0xE,
UV 0x11…, in u32 words).

## 4. Messages and responses

A **message** is a u32 id, optionally with a sender object and a target. They are queued and processed once
per frame, after the update (`ProcessMessageQueue`). **Targets:**

| Target | Where it goes |
|---|---|
| object | that object's responses for the message id |
| 0 | every package: its package responses (`PkgR`) and the objects listed in its `Targ` entry for that id |
| `0xFFFFFFFF` | the game (the host) |
| `0xFFFFFFFE` | package responses of every package |
| `0xFFFFFFFD` | package responses of the sender's package |
| `0xFFFFFFFC` | the sender's package: its responses and `Targ` objects |
| `0xFFFFFFFB` | the sound system |
| `0xFFFFFFFA` | input control (`DisableInputs`, `EnableInputs` for the sender's package) |

Events of a script send their message to the event's target (0 = every package, GUID = that object);
the message `0x1B3909AA` is special: it makes the target object the package's current button.

A response list is run in order. Responses **[decomp]**:

| Id | Meaning | Parameter |
|---|---|---|
| 0 | set the object's current script, time 0 | script id |
| 1 | post a message to FEng, target = the response target (a GUID in this package, or a special value) | message id |
| 2 | post a message to the game | message id |
| 3 | post a message to the sound system | message id |
| `0x100`+ | button control: set the active button; record or recall the current button; "do not navigate" and pass-control variants | GUID |
| `0x200`+ | package control: switch, push, pop packages, markers | package name |
| `0x300` | if the object's current script id equals the parameter, continue; else skip to the matching else/end | script id |
| `0x301` | the same, if it differs | script id |
| `0x500` / `0x501` | else / end if | |

Package commands (switch, push, pop) are **outgoing messages to the host** in this runtime: the host owns the
package list.

## 5. Buttons and focus

- A package has a **current button** (initially none). Setting it queues two messages for the old button
  (`0x55d1e635`, "lose focus", to the object and to sound) and two for the new one (`0xabc08912`,
  "gain focus").
- The host feeds input as a pad mask each frame (`set_pad_mask`); the engine turns it into the pad messages
  (accept, back, directions with repeat, …) and delivers them to the current button or the package, as
  [feng-input.md](feng-input.md) describes. `send_to_focus(package, message id)` is still there for hosts that
  map the input themselves.
- **Navigation between buttons is the engine's:** a direction moves the focus to the best button by geometry
  unless the button has the *do not navigate* flag or its response says so (feng-input.md section 3). A package
  can also set the current button itself (script event `SET_CURRENT_BUTTON`, response `0x100`).

## 6. Drawing

The runtime emits a list of drawable nodes; the host draws them. Rules **[decomp]**:

- **Order:** the sort key is `z = (parent context * (pivot + position)).z`, the object depth after its
  ancestors; objects with `z <= 0` are skipped. The sorter orders ascending, and the packages only make sense
  drawn **farthest first** (the tachometer face has z 200 and the needle z 10): so larger z is drawn first,
  and the depth-first list order breaks ties (a later object draws over an earlier one). An object with alpha
  0 is skipped; for a group, its whole subtree is. An object whose flags have bit 3 set (`0x8`) is not drawn on
  the PC.
- **Transform:** an image is the unit square (−0.5..0.5). `world = parent * T(position) * T(pivot) *
  R(rotation) * T(−pivot) * S(size)`. A negative size mirrors. Groups and leaves use the same product.
  Correction (2026-10-09): the current reference's
  `GenerateRenderContext` calls `MakeRenderMatrix`, including `S(size)` for groups too **[decomp]**.
  MainMenu's right-arrow group has x size -1 and its cursor groups animate their sizes **[verified files]**;
  ignoring group size loses both mirroring and pulsing.
  UVs are used as stored.
- **Colour:** the object's colour times the colours of its ancestors, per channel `(a * b + 128) >> 8` on
  0..255 values (this runtime uses `a * (b + 1) / 256`, which keeps 255 exact). The vertex colour divided by 255 modulates the texture. The blend mode comes from the
  texture (`AlphaBlendType`: 1 blend, 2 additive). Additive uploads must first multiply RGB by
  the texture alpha, then set output alpha to zero for the premultiplied compositor **[ours]**.
  `IconSelection_Glow` is a DXT3 additive texture with white RGB in transparent texels **[verified files]**;
  discarding its alpha without multiplication draws the entire rectangle.
- **Strings:** glyph quads placed around the string origin. Horizontal origin: left 0, centre −width / 2,
  right −width; vertical: centre −height / 2, bottom −height. A maximum width (`Sw`) squeezes the line
  horizontally unless word wrap is set. Leading is `Sl` times the font's leading scale. Fonts and glyphs:
  [formats/frontend.md](../formats/frontend.md#fonts-fengfont-decomp--verified). The text of a string whose
  "not localized" flag is clear and whose label hash is known comes from the language table; otherwise the
  object's own text is shown.
- **Multi images** use texture 1 as a mask. The object data ends with a pivot (`x`, `y`, fractions of the
  texture) and a rotation `z` in degrees, which the game sets to fill a gauge (`FEngSetMultiImageRot`: the
  nitrous bar, the redline, the heat and engine temperature meters). The mask's coordinates are rotated about
  the pivot (with the pivot at the centre, a point `P` of the picture samples the mask at
  `c + R(P - c)`, `R = [[cos, sin], [-sin, cos]]`, `c` the centre of the mask, all in pixels); the picture is
  drawn where the mask is. **[inferred]** The textures of the HUD's gauges settle the blend: the mask is a half
  ring in its alpha channel (black colour) and the picture is the same half ring, so the picture's alpha times
  the mask's alpha leaves an arc that shrinks as the rotation goes from 0 to 180 degrees, which is what a
  gauge needs. The original's blend is in platform code that is not in the decompilation. The mask also has a texture
  rectangle (the object's UVs for texture 1, the unit square for a gauge): the minimap's pieces move it so a disc
  stays still on the screen while the picture scrolls under it ([hud-minimap.md](hud-minimap.md)); where the
  rectangle leaves the mask texture the mask is 0.
- **Clip regions** are not used (the engine's clip path is empty).

## 7. The host interface

The runtime is plain data in and out; it knows nothing about rendering or the game.

- **Load:** a parsed package becomes a running package (assigned an id). Resource names are returned so the
  host can bind textures and fonts.
- **Update:** `update(seconds)` advances scripts and processes the message queue.
- **Bind (data in):** by object name hash and package: set visibility, string text (or a label hash), the
  rotation about z, the colour or alpha, a texture hash, a position; post a message (by id and optional
  target); switch an object's script by id.
- **Outgoing:** `take_outgoing()` returns the messages sent to the game, the sound system, and the package
  commands (switch, push, pop).
- **Read (tree out):** a retained flat tree of nodes with id, parent, kind (group, image with texture and UV,
  text with the label or text, font and justification), local transform, size, colour, alpha, visibility,
  z-order, and the computed world transform, accumulated colour and draw order.
- **Input:** `set_pad_mask`, `set_control`, `set_focus`, `send_to_focus`, `post_to_package`.

## 8. The in-game HUD (`HUD_SingleRace.fng`)

372 objects, 69 resources; the gauge cluster is at the bottom right. The game finds objects by name hash and
switches each element on by a feature mask (`FEngHud::DetermineHudFeatures`: the gauges setting turns on the
speedometer, the tachometer, the nitrous gauge and, with a turbo, the turbo gauge):

| Object (name) | What the game does with it |
|---|---|
| `SpeedometerGroup` (0x941fff09) | shown by the speedometer feature (mask `0x8000000`) |
| `SPEED_DIGIT_1` … `_3` | printf `%d` of the integer parts of the speed (truncated, not rounded); a leading zero hides the digit |
| `3rdPersonSpeedUnits` | string with label `0x8569a25f` (KM/H) or `0x8569ab44` (MPH) |
| `GaugeCluster` (0x5164d4ea) | the group of the tachometer features (mask `0x2`) |
| `3rdPersonNeedle` | image rotated about z by `66 + 228 * clamp(rpm / scale)` degrees (below) |
| `3rdPersonGear` | string `1`…`8`, `R`, `N`; black, alpha `0x88` while a gear change is in progress, `0xFF` otherwise |
| `Shift_light` | script `GREEN` (0x02DDC8F0) when the shift potential is above 1 (up, good, perfect, miss), `INIT` otherwise |
| `TAC_Lines_7500` (0x309878bc) | its texture swapped to the tachometer face `<scale>_LINES_<skin>` |
| `RPM_REDLINE` (0xcdfce1b0) | multi image whose mask is turned to the red zone (table below) |
| nitrous group (0x87c38e97), icon (0x27ddf583), bar (0xedfb6d37) | shown with the nitrous feature (`0x800`); the bar is a multi image whose mask is turned to `175 - 175 * nos` degrees (`nos` 0..1, full = 0) |
| `TURBO_GROUP`, `3rdperson_TurboDial` | shown with the turbo feature (`0x20000`); the dial turns to `-(-45 + 90 * (psi + 20) / 40)` degrees |

**Scale.** The engine's `MAX_RPM` picks the face and the sweep: below 7000 the 7000 face, below 8000 the 8000 face,
below 9000 the 9000 face, else 10000 (`ChooseMaxRpmTextureNumber`, so an engine with `MAX_RPM` 8000 has the 9000
face). The needle's fraction is `rpm / scale`.

**Red zone.** The mask of `RPM_REDLINE` is turned to an angle chosen by the `MAX_RPM` band and the red line (the
bigger the angle, the shorter the red zone); the first row whose red line is reached applies:

| `MAX_RPM` below | red line from, angle | | | | else |
|---|---|---|---|---|---|
| 7000 | 6500: 164.5 | 6000: 149.5 | 5500: 131.5 | | 113.5 |
| 8000 | 7500: 165 | 7000: 152 | 6500: 138 | 6000: 123 | 110 |
| 9000 | 8500: 166 | 8000: 154 | 7500: 140.5 | 7000: 127 | 115 |
| above | 9500: 167 | 9000: 156 | 8500: 145 | 8000: 134 | 123 |

**Nitrous icon.** Each update the game tells the gauge the tank level: at 0 or below the icon plays `INIT`; below
the previous level (draining) it plays `0x77031C70`; otherwise `0x03826A28`.

**Shift potential.** For a car on the automatic gearbox the potential is "up" when the engine speed matched to the
wheels has reached the gear's shift-up point and there is a higher gear (`FindShiftPotential`), and it is cleared
while the clutch is not engaged. **[inferred]** The tachometer is also told whether the car has traction and
zeroes the potential for a frame after a gear change; the build does neither (the callers are not in the
decompilation).

**Not driven yet** (hidden): the radar and its detector, the pursuit, heat, busted and cost-to-state boards, the
milestone and race boards, the countdown, the infractions, the speed breaker meter, and the engine temperature
gauge of the drag HUD. They need the race and pursuit state of milestone 7. The minimap has its own spec,
[hud-minimap.md](hud-minimap.md).

Messages: `WIDESCREENMODE` 0x62ED04EC, `NORMAL_MODE` 0x53EC068C, `FADEIN` 0xBCC00F05, `FADEOUT` 0x54C20A66.
The HUD also has package responses that fade parts in and out; the runtime runs them as any other.

## How to check it

1. Load `HUD_SingleRace.fng` and print the tree: 372 objects, `INIT` scripts applied, the gauge cluster at the
   bottom right of a 640 × 480 screen.
2. Post `FADEIN` and check alpha rises over 0.625 s; post `HIDE` and check the object stops drawing.
3. Compare a screenshot of the HUD with the game's at the same resolution and speed (digits, needle angle).
4. Measure the ticks per frame in the game (a script of known length against a stopwatch).
5. Check the order of overlapping objects (the speed digits over their backing).
