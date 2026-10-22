# Destroyed police coasting tires provenance

PC executable SHA256 80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c:
Ghidra identity confirmed before bounded inspection of Simple Tire::UpdateLoaded
at 0x69f020. The no-powered-slip/no-handbrake branch confirms the angle ramp,
brake-dependent force limit, rolling wheel velocity, low-speed brake law, absolute
forward-speed corner drag and rear-only unbraked surface gates. An isolated,
hash-checked x86 execution matrix supplies independent numeric outputs covering
forward/reverse, sideways motion, low speed, ordinary brake, engine braking and
non-unit surfaces. Inputs use 4000 N load, 0.35 m radius, grip/static/dynamic
1/1.2/1.1, grip boost 1.2, traction boost 1 and corner drag 0.5. These are synthetic
routine inputs, not captured full-car trajectories or guessed installed masses.

The CC0 reconstruction SuspensionSimple.cpp corroborates the control/drive split
and declares regular brake scale 10. The existing racer adapter uses scale 4;
the generic policy receives their ratio from the game adapter. No original binary
bytes, tables or installed assets are included. Selected modified executable
SHA256 bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c
is a different full file; any matching mapped ranges establish static identity
only. No live injected mod behavior is asserted.

The saved PC task wrapper at 0x6b2870 lacks a function definition. Its rear
overshoot grip correction is not claimed as PC-verified or implemented here.
See the contract for remaining Simple physics gaps.

Offline PE mapping found the loaded routine's 1741-byte range, the angle wrapper's
127-byte range and the three referenced brake/lateral gain constants byte-identical
between those two files. This does not extend to arbitrary executable ranges.

The matrix includes explicit brake-dependent saturation at brake 0.1: lateral
-5991.7788 N and longitudinal 2675.2664 N after corner drag, traction 0.81057614.
