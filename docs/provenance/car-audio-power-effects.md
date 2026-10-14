# Power-effect audio provenance

Implementation is derived from the
[behavioral specification](../specs/car-audio-power-effects.md), written
before the adapter. Native PC routine identity was confirmed with Ghidra
program metadata, including executable SHA256
80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c.
Private routine dumps, captures and assets stay outside the repository.

Read-only references: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
CC0 decompiled CARSFX_Nitrous.cpp, CARSFX_Shifting.cpp,
SND_GEN/MAIN_AEMS.h, SND_GEN/ENGINES_AEMS2.h, EAXSoundEnums.hpp.
The matching PC constructors independently establish class parameter
order and setter bounds. PC nitrous vtable/update/stop establish active
edge lifecycle and explicit stop parameter plus destruction request.
Its per-update vtable target was not a recognized Ghidra function;
bounded original program-memory bytes establish volume/azimuth/decoded
pitch/filter updates and their bounds. PC shifting constructors and
reference enum independently establish up id1/down id0, contrary to
the previous guessed direct-sample path. PC sweetener constructors
establish IDs, class/RPM/gain bounds and sequential Q15 multiplication.

An independent isolated execution of original PC instructions established
seven synthetic oracle cases: three turbo-constructor clamp/interior
vectors, nitrous stop's mixer-active clear and class stop word, and three
clunk gains read from the original cosine table. All513 consumed cosine
entries equal the independently computed1022-divisor formula. Only the
constructor's external class-registration callback was stubbed. This
executes no original game process and establishes neither graph/audio
timing nor RNG equivalence. Public regression expectations derive from
those observed vectors; private bytes/addresses remain private.

Original driving captures positively observed all four authored classes.
That establishes their use, not exact event alignment or waveform parity.
No claim is made that capture voice-start counts equal driving events.
Tests cover deterministic parameter arithmetic and actual installed-bank
voice/PCM lifecycle across render rates. Test implementation is distinct
from a recorded successful validation run. Original sound timer/RNG,
specialized camera POV states, nitrous type/upgrade selection and environmental
DSP remain explicitly unresolved.

The additional forced-induction/reverse adapters use CARSFX_Turbo.cpp,
SND_GEN/TURBO.h and CARSFX_Shifting.cpp as behavior references.
Matched PC turbo constructor independently establishes six class words
and every bound; matched PC reverse creation/update establishes class0,
gain/RPM/azimuth bounds and constructor defaults. SFXCTL_Physics.cpp,
EAXTunerCar.cpp and EAXCar.hpp establish that the POV word differs from
the dynamic-mixer camera state: bumper0, hood1, chase2/3. The existing
reverse gain denominator is supplied by the physical vehicle, with no
engine-audio maximum substitution. Blowoff selection still shares the
existing deterministic controller; native RNG equivalence is unverified.
