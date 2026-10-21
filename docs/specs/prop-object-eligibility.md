# Prop object collision eligibility

Source-backed native object collision eligibility is separate from world and
ground collision eligibility. A rigid prop participates in object broad phase
only when it has collision primitives, its linked rigid-body record does not
set `NO_OBJ_COLLISIONS`, and its body is not frozen. An asleep or attached body
remains eligible: approaching vehicles and other props must still hit it.

Both endpoints of a prop pair must satisfy this rule. A moving body excluded
from the grid must not initiate collisions against eligible grid members.
Disabling object collisions does not suppress gravity, ground support, rendering
or world contacts. Primitive object-channel and enabled flags remain separate
narrow-phase filters.

Verification covers disabled objects as both initiating and receiving endpoints,
frozen and asleep states, and independent ground support.
