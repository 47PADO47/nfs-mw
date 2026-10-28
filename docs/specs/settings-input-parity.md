# Settings input and prompt transitions

## Replacement footer artwork

**[verified: installed assets and remake regression]** `Options.fng` and `Pause_Options.fng` contain a native key image
named `0xe795717d`, directly under their animated footer groups `0x84bcf2b5` and `0xb3a69e89`. It is outside the
caption groups hidden by the replacement footer. Its visibility can overlap the replacement input hints.

**[verified: installed assets and presented exit frames]** Native `PC_key_enter` and `PC_key_esc` images use texture
hashes `0xd7b6edb7` and `0x1bb3c514`. Their parent groups differ between main and pause settings. The main Enter key
is hidden while settled, but the exit script shows it again during Back, outside the replacement footer's caption
suppression. Its drawn position `(-166, 214)` matches the observed overlap with the replacement Adjust hint.

**[ours]** When these settings footers receive replacement hints, suppress these native key images in the presented
tree for both keyboard and controller. Identify the PC images by texture within these two settings packages; do not
depend on their differing parent hierarchies. Keep runtime animation, footer backgrounds and other artwork intact.
The replacement hints continue to follow the selected device, its bindings and the authored footer fade/transform.
The same artwork outside these settings packages is not sufficient reason to hide an object.

**[verified: follow-up asset inspection]** The PC key's parent owns a separate caption and striped box. Hiding only
the key leaves that box visible during Back. Suppress the complete PC hint in presentation, including descendants;
do not suppress the shared replacement footer bar. Main and pause packages use different PC hint parent groups.

Regression coverage must inspect the actual presented tree through entrance, Back and return, including consecutive
keyboard/controller presentations. A settled screenshot alone does not prove transition correctness.
