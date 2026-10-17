# PC engine redline eligibility

This supplements [engine-sound.md](engine-sound.md) section 7. The earlier
GameCube reference does not include the PC build's highest-gear guard.

## Evidence

**[verified, static PC disassembly]** The v1.3 executable with SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`
checks the current gear against the car sound state's highest gear before
entering the redline state. Equality skips entry, even above 9800 audio RPM.
The state constructor obtains that highest gear from the transmission ratio
array's length minus one. Gear IDs include reverse 0 and neutral 1.

Other entry requirements, the 9800 threshold and the 15%/85% replacement fades
remain as documented in the main spec. The highest-gear check guards entry;
release still follows RPM falling below the threshold or a shift starting.
Neutral and lower forward gears retain the limiter sound. This is distinct
from the physics rev limiter, whose torque/RPM behavior is unchanged.

## Host contract and regression

**[decision]** Supply the installed transmission's highest gear alongside each
physics sound tick. Hosts without transmission telemetry leave it unknown and
retain the preceding behavior. Do not infer it from road speed, car name or an
assumed number of gears. An upgrade can change the ratio table.

The regression uses the installed Lotus Elise with its maximum performance
package and Junkman parts, the resolved engine recordings and highest gear.
At full RPM and 366 km/h it must retain normal engine layers with a zero
redline replacement level. Holding the same RPM one gear lower must still
engage the replacement. Software rendering verifies audible, finite output;
this does not establish a listening comparison with the original at that speed.
