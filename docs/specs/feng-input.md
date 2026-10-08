# FEng input (pad events, focus, navigation)

How the FEng engine turns controller state into messages for a package, and how it moves the focus
between buttons. The rest of the runtime (scripts, messages, drawing) is in
[feng-runtime.md](feng-runtime.md); what a screen does with these messages is in
[frontend-menus.md](frontend-menus.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/`: `FEng/{FEngine,FEJoyPad,FEButtonMap,FEPackage,FEObject}.{cpp,h}`,
  `FEng/FEKeyTrack.h`, `Frontend/FEJoyInput.cpp`, `Frontend/FEngHashes/*`. Read for understanding; no code
  copied. Provenance: [provenance/frontend-menus.md](../provenance/frontend-menus.md).
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). Everything is **[decomp]** unless
  marked. The message ids below are `bStringHash` of the upper-cased name (checked: `PAD_ACCEPT` hashes to
  `0x406415E3`) **[verified]**.

## 1. The pad mask

The game turns each controller's frontend actions into a 32-bit mask once per frame (`cFEngJoyInput`):

| Bit | Button | Bit | Button |
|---|---|---|---|
| 0 | up | 6 | start |
| 1 | down | 7 | left trigger |
| 2 | left | 8 | right trigger |
| 3 | right | 9..14 | buttons 0..5 |
| 4 | accept | | |
| 5 | back (cancel and its alternate) | | |

The engine keeps, per pad, the mask of this and the last frame and a *held count* (ticks) per bit:
**pressed** = set now and not before, **released** = the reverse, **held** = set in both frames (its count
grows by the frame's ticks; a press starts at 0).

## 2. Buttons and messages

A package with input enabled and a non-empty control mask is processed every frame **before** its scripts.
`current` is the package's current button (none at the start; the game or a script sets it).

**Bits 4 and up (accept, back, start, triggers, buttons)**, in bit order:

- *Pressed* sends the bit's message (`PAD_ACCEPT`, `PAD_BACK`, `PAD_START`, `PAD_LTRIGGER`, `PAD_RTRIGGER`,
  `PAD_BUTTON0`..`9`):
  - accept: if `current` has a response for `BUTTON_PRESSED` (`0x0C407210`), that message goes to `current`
    (and to the sound system); otherwise, if the package has a package response for `PAD_ACCEPT`, `PAD_ACCEPT`
    goes to the package responses (target `0xFFFFFFFD`) and to sound;
  - every other bit: the message goes to `current` if it has a response for it, else to the package
    responses if the package has one (and to sound either way).
  The pressed button is remembered so the release goes to the same button.
- *Held* triggers also send `PAD_LTRIGGER_HELD` / `PAD_RTRIGGER_HELD`, the same way.
- *Released* sends the `_RELEASED` message (accept: `BUTTON_RELEASED` `0x936A6A7F`) to the remembered button if it
  is still `current`, and to the package responses if the package has one.
- With the package flag *start equals accept*, a start press also counts as an accept press.

**Directions (bits 0..3).** Eight cases are tested in this order, the first that fires wins and ends the
frame's direction processing: up+left (up-left), up+right, down+left, down+right, then up, left, down, right.
Messages `PAD_UPLEFT`, `PAD_UPRIGHT`, `PAD_DOWNLEFT`, `PAD_DOWNRIGHT`, `PAD_UP`, `PAD_LEFT`, `PAD_DOWN`,
`PAD_RIGHT`.

A case **fires** when its bits were just pressed (a diagonal needs both pressed in the same frame), or when the
shorter held count of its bits reaches the repeat time: **20 frames (320 ticks) for the first repeat, 120 ticks for
the following ones** (a case that repeated once is remembered as *fast* until it is released). After a repeat the
bits' held counts drop by the repeat time, so the next one comes a repeat time later.

On firing, with `msg` the case's message:

1. `current` exists and has a response for `msg`: queue `msg` to `current`. The focus then moves by geometry
   (below) unless `current` has the *do not navigate* flag or that response contains a button-control response
   `0x104` ("do not navigate").
2. `current` exists and has no response for `msg`: the focus moves by geometry (unless the flag), and `msg`
   goes to the package responses.
3. No `current`: `msg` goes to the package responses if the package has one.

In every case with a `current` button the message also goes to the sound system. If a new button was found,
every button remembered as pressed gets its release message, then the new button becomes current (queuing
`BUTTON_UNHIGHLIGHT` `0x55D1E635` for the old and `BUTTON_HIGHLIGHT` `0xABC08912` for the new, each also to sound).

## 3. Navigation by geometry

Buttons are the objects with the *is button* flag (bit 28 of the object flags) and without *ignore button*
(bit 26). *Do not navigate* is bit 19.

From the position of `current` (the object position, taken through its render context) the engine scores
every other button for a direction `d` (one of eight unit vectors, up = (0, -1), screen y down):

```
delta = pos(b) - pos(current); dist = |delta|; if dist < 0.0001 skip
a = dot(delta / dist, d); if a >= 0 { a = a * a }
score = if a >= 0.25 { (1 - a) * 200 + dist } else { 1500 }
```

The lowest score wins and is accepted if below 1500. With wrapping (off by default) the search is repeated from
a position shifted by one screen width or height, and the nearest wrapped candidate may win.

## 4. Other rules that matter

- The *input enabled* state of a package (`button 0x101`, `DisableInputs` / `EnableInputs`) stops all of the
  above while off. A package can also be given a control mask of 0.
- `QueuePackageMessage(msg, package, none)` from the game is a message with the target "sender package"
  (`0xFFFFFFFC`): the package's responses and the objects its `Targ` table lists for `msg`.
- A script event with message `SET_CURRENT_BUTTON` (`0x1B3909AA`) makes its target object the current button.

## How to check it

1. Unit tests on a synthetic package: a row of three buttons, right moves the focus and queues the focus messages;
   hold repeats after 320 then every 120 ticks; accept goes to `current`'s `BUTTON_PRESSED` response.
2. `nfsmw view-screen MainMenu.fng --ui-script ...` and a screenshot per step (see the menus spec).
3. Against the running original (not done): log the messages a package receives for a held direction.
