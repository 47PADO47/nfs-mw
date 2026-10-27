# Experimental lamp surface

This is an artistic prototype, not original-game behaviour. It is enabled for
Custom car shading by the F8 **Enhanced vehicle lamps** option.
Original and simple shading retain their existing surface response.

Only named headlight and brake-light texture slots with installed ON artwork
receive the experimental emission. The active state comes from
the existing vehicle lamp variants. Broken left and right headlight bits select
their emission-off state independently, as brake-light variants already do. If
OFF artwork is missing the original texture fallback remains visible, but the
new emission still turns off. A missing ON texture falls back to the ordinary surface. Glass and window slots are
not treated as emissive surfaces. A named lamp-glass solid is also excluded when
it reuses a lamp-body texture slot. Vehicle models and textures are not edited.

The shader derives a soft emission weight from bright features in the ON
texture, combines it with surface colour in linear light and converts back to
the existing encoded scene representation. Red-dominant brake artwork gets a
redder response while neutral details are not tinted red. This is a brightness heuristic,
not an ON/OFF difference mask or a reconstruction of lens depth. It can select
painted highlights that are not bulbs. The result needs model-specific visual
checking, including replacement textures, reflections, damage, HDR and SDR.
An additional small white/warm headlight core and faint halo, and a restrained
red tail halo, use the installed marker position, facing and lamp state. They use ordinary depth-tested effects,
and disappear when the associated lamp is off or damaged. Original flare
profiles remain present, and original car shading does not add the new core.
It does not illuminate nearby surfaces or add headlight beams. Ray-traced
reflection hits do not reproduce the new emission.
