# Emergency car lights

Specification: [emergency-car-lights.md](../specs/emergency-car-lights.md).

Original executable SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`
was confirmed with Ghidra program identity before bounded read-only queries. `RenderFlaresOnCar`
at `0x742950` confirms colour enable/broken masks, simulation time plus a per-car phase, inlined
colour pulse math, headlight modulation and no additional emergency night gate. The standalone
flicker helper at `0x738300` confirms the 12-radian frequency and marker phase offset. The saved
project does not define the standalone second helper at `0x738340`; its relevant operations are
present in the analysed renderer. Startup stores at `0x88B7B0` initialize the blue phase and periods.
Independent bounded inspection of PC `eRenderLightFlare` at `0x505380` confirms that directional
intensity gain is conditional on marker flag `4`; the emergency markers constructed with flag `2`
bypass that gain. This does not establish every other meaning of flag `2`.

Offline PE mapping compared those functions and the 48-byte startup region against executable
SHA-256 `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`: the respective
2617, 56, 247 and 48 bytes match. Referenced initialized constants also match. This establishes static
file equivalence for these ranges, not equivalence of injected modifications in a running game.

The standalone second helper was also executed in isolated x86 memory with synthetic time/colour/
ordinal arguments after executing its three original startup initializers. No game process was
launched or attached. Twenty-seven helper results across nine times and three colours establish the
numeric pulse reference. Mathematical trigonometry differs from the original angle table by less
than `0.000087` across that matrix; the red/blue regression uses a `0.0001` absolute tolerance.

CC0 reference material read for understanding: dbalatoni13/nfsmw,
`World/CarRender.cpp` (`RenderFlaresOnCar`, flare marker construction and car phase initialization),
`World/VehicleFX.cpp`, `Generated/AttribSys/Classes/light_flares_cg_hash.h` and
`Physics/Behaviors/SoundCar.cpp`. The latter independently shows damaged police lights can request
the dying siren state; this rendering implementation does not change siren audio behavior.

Installed emitter inspection found 0.01-second lifetimes for all three cop-light colours. The current
particle runtime advances newly born particles during a 1/60-second simulation step, so these
particles expire before drawing. Adding the independently specified native flares addresses that
missing visual path without changing installed particle lifetimes or global particle integration.

The implementation follows the behavioral specification with mathematical trigonometry and installed
profile readers. It contains no executable bytes or copied original implementation. Original table
trigonometry, full marker traversal order, character/world dynamic lights and reflection passes have
not been established as pixel-identical by this change.
