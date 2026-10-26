# Player gauge presentation

The normal HUD's main analog needle is the tachometer. The standard speedometer
uses digits. See [provenance](../provenance/player-gauge-presentation.md).

## RPM producer

The original tachometer receives the engine's displayed RPM, not instantaneous
angular velocity converted to RPM. The engine already supplies its asymmetric
deceleration limit and weighted display filter. Preserve raw physical telemetry
for simulation and diagnostics; do not add an arbitrary needle damping constant.

Audio also supplies visual RPM, including the existing pre-race physics-RPM gate
and post-countdown transition described in [engine sound](engine-sound.md#44-visual-rpm-the-tachometer).
Return the existing controller output to the player HUD. When that voice has no
valid output, fall back to engine display RPM. Audio absence must not freeze the
tachometer. Do not feed this visual value back into engine torque or launch strength.
Staging and drag use engine display RPM directly; normal driving uses the audio
feedback when available. The audio controller's physics input also starts from
engine display RPM, rather than raw engine angular velocity.

The standard speed readout uses physical body-speed magnitude. It does not use
signed forward speed or the transmission-derived speedometer calculation.

The remake may interpolate completed display snapshots using its existing render
phase, as it does body poses. This is presentation between known states, not
extra original-game damping or a change to physics. Snap the history on placement,
hardware replacement and authoritative corrections; prediction restores it.

## Perfect-launch needle color

While the caller enables standing-start staging and the engine supplies a first-
gear launch band with positive lower edge and width, the HUD range includes both
endpoints: `lower <= displayed RPM <= lower + width`. Physical launch strength
continues to exclude its endpoints. Free roam and released/aborted staging have
no launch-range highlight.

The original normal and drag tachometers complement the needle RGB when this
range flag changes, forcing opaque alpha. Complement it again on exit to restore
RGB. This is not a new green band. Compose the highlight after authored scripts
and optional custom gauge RGB so custom colors cannot overwrite it. Restore the
current custom/authored RGB when the range ends or the custom color changes.
Other gauge parts, including the turbo needle, keep their own colors.

## Acceptance limits

The raw-RPM HUD route and missing color path are concrete source defects. A
passing binding test does not close the reported live Golf needle-jitter issue.
Updated local builds require playtest acceptance before PR publication. Native
countdown and after-GO comparison remain separate from component gauge tests.
