# Stock quick-race driving evidence

PC program identity was checked before new static queries. The inspected
executable SHA-256 is
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.

`SuspensionSimple::DoWheelForces` at `0x006ac480` checks racer class 3 and excludes
driver style 1 (drag). Its steering-drag coefficient is a base 0.23 plus cheat
times 0.27. The reference declaration in the CC0
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) file
`Physics/Behaviors/SuspensionSimple.cpp` corroborates that branch. No translated
implementation is copied. The host previously reset ordinary tires to 0.15
without applying this per-racer coefficient. Correcting it is not evidence that
the rest of Simple tire dynamics already matches the executable.

The existing driver specification establishes physical-grade acceleration and
zero grade while emulated. Source inspection found the race action always passed
zero despite having the body forward vector available at its caller.

Two complete modded-PC captures on 2026-11-05 use event 14.2.3, Hard, six rivals,
no traffic, police Off, stock Carrera GT, no Junkman and unchanged tuning. The
user confirmed the hardware. Executable SHA-256 is
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
The new bounded collection-parent reader resolves every participant's base model.
Off selects pairs of Carrera GT, Gallardo and DB9. On selects pairs of Clio,
Mustang GT and Monaro. These are different comparison fields.

The recorded PID integral, averaged derivative and error reproduce enabled
quick-race outputs with maximum normalized residual below 0.000001. The Off
capture records 51,274 disabled/non-forced calls and 2,034 enabled/forced calls.
The On capture records 52,339 enabled/non-forced and 289 enabled/forced calls.
This independently validates the previously inspected forced/off-world branch.

The CC0 `AI/Common/AIVehicle.cpp` reference records errors independently of the
selected switch and initializes its sample timer with a random phase below one
second. The host must preserve separate deterministic random streams. This does
not establish the native selection algorithm, random sequence or career adaptive
branch. Further inspection of `Physics::Info::EstimatePerformance` at
`0x0067c570` agrees with the existing
[garage rating contract](garage-performance-bars.md): seven weighted package
fractions, normalized stock/max endpoints and the Junkman adjustment. Reuse that
implemented calculator for ordinary quick-race catch-up's human top-speed rating.
Native cached-rating values are not included in these telemetry captures, so
their exact numeric runtime agreement remains a validation boundary.

Captures have audio discontinuities and uncalibrated audiovisual onset. Their
telemetry supports this driving comparison; it does not establish frame-exact
presentation parity. Private records retain original files, source snapshots,
hashes, settings and diagnostic limitations separately from public implementation.

Staging player-engine samples retain a roughly 1/144-second simulation interval;
consecutive same-racer catch-up calls retain roughly 1/72 second. Engine hooks
normally throttle observations, so only the unthrottled staging player samples
establish the engine interval. This matches the existing host's supported 144 Hz
comparison mode, rather than its normal 60 Hz default. Preserve both results.
