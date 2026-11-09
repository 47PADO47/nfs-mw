# The rider kept out of the bike's body: provenance

Spec: [ride6-rider-clearance](../specs/ride6-rider-clearance.md). Code: `crates/nfsmw/src/scenes/world/drive/rider/{clearance,pose}.rs`,
`mods/ride6-bikes/src/rider/body_section.rs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The user's own export of each bike (`C:\r6work\bikes\<bike>\glb`, `parts.json`) | The vertices of the body-role parts (the chassis, the exhaust, the hand guards) in the bike's frame, and which bone each is skinned to | Measured by the converter; the grid is the only thing kept, in `contacts.json` of the installed mod, nothing committed |
| The rider's physics asset as converted (`ragdoll.json`, spec [ride6-rider-fall](../specs/ride6-rider-fall.md)) | The capsules (bone, centre, axis, half length, radius) that stand for his body | The user's own export |
| The animation correction table's bike row (`contacts.json` `correction`: `knee`, `elbow`, `aero_tank`, ...) | Named as RIDE 6's own answer to the same fault; **not read for any number** | Facts about the user's data asset; not played |

## What is a choice

Everything in the constants table of the spec: the cell, the extent, the tolerance, the bounds, the lever and the torso rule are choices of the method. The method (swing the knee
out about the hip-to-foot line) is what a rider does and what the two-bone solve's pole is for; no decompiled code was used.
