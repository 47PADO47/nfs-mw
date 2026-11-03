# UI repair batch R8: My Cars entry, carbon hoods and the car switcher

Repair candidates from the R7 playtest follow-up. User acceptance remains open.

## Changes

1. **My Cars keeps the crib on screen.** The menu backdrop drew its room only once a car was up, and My Cars uploads its
   car after its catalogue is read, so the screen went dark in between. The garage's backdrop now draws the room from the
   front end's camera before the car arrives, and that camera keeps moving.
2. **Carbon hoods show carbon fibre.** A carbon hood is the same solid as its painted twin. The original draws it through
   a second texture table in which the car's `<CAR>_SKIN1` paint placeholder shows `CARBONFIBRE`. The remake's
   assembly did not do this, so a carbon hood looked like the painted one. A hood part with the `CARBONFIBRE`
   attribute now has those placeholders on its solids redirected to `CARBONFIBRE`. **R9:** the hood's `CARSKIN` groups
   also take the car's `CARBONFIBER` light material instead of the paint's; lit as paint, the weave was lost under the
   paint's reflections and a carbon hood looked like a darker painted one.
3. **The career car switcher wraps.** Left on the first car goes to the last, and Right on the last goes to the first.
   The car lot's lists keep their clamped stepping.

## Evidence

- Installed hoods (Carrera GT and GTO): every carbon hood names the same solid and textures as its painted twin. The new installed
  regression checks that a GTO carbon hood draws `CARBONFIBRE` and its painted twin does not. It passes on the modded
  and the unmodified installs.
- The wrap follows the playtest report. It has not been confirmed in the executable.

## Open

- **Physical check.** The My Cars entry, a carbon hood's look under the menu light, and the wrap need checking in play.
- **Carbon placeholders.** Two further entries of the carbon table have no recovered placeholder names. The hoods checked
  (Carrera GT and GTO) do not use them.
- **Baseline.** `installed_camera_noise_preserves_each_surface_pair` fails on both installs in an unchanged file.
