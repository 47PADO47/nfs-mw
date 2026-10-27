# Player gauge presentation provenance

- Date: 2026-10-26.
- Spec: [player gauge presentation](../specs/player-gauge-presentation.md).
- Original PC program identity rechecked: clean analysis SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
  selected memory-patched executable SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- HUD consumer `006f13f0` supplies selected RPM to ITachometer. It clears the
  perfect-launch-range flag, obtains the race engine band in the allowed context,
  then applies inclusive lower/upper tests. The normal visual update `0057a6e0`
  maps its stored RPM directly to the normal 66-degree rest/228-degree sweep.
  Its update does not add another angular smoothing filter.
- The staging predicate for the color range is IVehicle::IsStaging. Raw stack
  dataflow resolves the decompiler's aliased local to IVehicle before its slot
  `+20` call; the slot points to the staging-byte getter `00688110`.
- The same HUD consumer uses IEngine display RPM while staging or in drag, and
  otherwise queries ICarAudio through `006eba50`. IEngine getter `006a03a0`
  reads full-engine offset `+180` (display RPM). ICarAudio getter `006a3f20`
  reads full-SoundCar `+7c` (audible RPM). SoundCar's service also obtains its
  physics RPM input from IEngine::GetRPM before normalization.
- Speed HUD call `006f14f9` selects IVehicle slot `+90`, getter `006881b0`,
  reading physical speed at interface `+70`. Slot `+8c`, getter `006881a0`,
  reads the distinct speedometer cache at `+78` and is not this HUD source.
- Normal needle transition block `0057a870..0057a8f2` complements current RGB and
  forces opaque alpha on range entry and exit. Drag has the corresponding
  operations at `0057d28f` and `0057d2ca`. Original instructions and decompilation
  stay private. Existing launch provenance records the range and strength rules.
- Source-path audit: CarSim physical telemetry previously called Vehicle::rpm,
  which returns instantaneous engine RPM. Drive passed it to HudState directly,
  bypassing Powertrain::rpm_display. The audio engine already exposed visual RPM
  but the application did not return it to the HUD.
- Existing engine-sound section 4.4 records the original audio visual-RPM path.
  Presentation interpolation is a remake scheduling choice, not evidence of a
  second filter in the original tachometer. Exact live gauge feel remains open.
