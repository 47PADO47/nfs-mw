# Experimental Visual Treatment

The native renderer can apply the original PC midday `visuallook/default` colour
profile, loaded from the user's base-game install. It uses the profile's tint,
desaturation and black/colour curves, and the installed screen gain map. Assets
are read at runtime; no textures or extracted tuning data ship in the repository.

Video options offer **Off**, **Subtle** (half-strength blend) and **Original**
(the full midday profile). The stored/CLI values remain `off`, `low`, `high`:
`--visual-treatment high`, `NFSMW_VISUAL_TREATMENT=high`, or
`visual_treatment = "high"`. The console accepts `visual_treatment high`.
The default is off and graphics presets leave this experimental preference alone.
Bloom remains an independent setting. No Over Bright glow pass is included.

Local scene luminance is downsampled to quarter resolution and blurred before
the curves are applied, then composited before bloom, tone mapping and AA.
The HUD and menus are unaffected. No temporal history or animated noise is used.
Missing or invalid look data skips grading and preserves HDR for later effects.
Luminance targets use the renderer's validated format fallback on non-HDR adapters.
The Bevy renderer hides this unsupported option and retains its saved preference.

This is a midday proof of concept, not complete original rendering parity:
dynamic tunnel exposure, time-of-day interpolation and pursuit/cinematic look
selection remain open. Existing lighting differences, including the port's
bright sky, can still cause highlight clipping at full strength. Subtle offers
a gentler comparison while those systems are implemented.

See [the spec](specs/visual-treatment.md) and
[provenance](provenance/visual-treatment.md) for sources and the colour equations.
