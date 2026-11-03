# Authored coloured-image gradients

The existing [frontend format](../formats/frontend.md) describes type 9 images with
four corner colours after their UV rectangle. The renderer previously discarded these
colours and drew every vertex with the uniform object tint.

**[verified, installed PC assets]** The pause background's mirrored side panels are type 9,
use BasePoly and have black vertex colours with alpha fading from zero to 200. The central
panel has black corners with alpha 200 throughout. The enclosing group animates opacity.
Ignoring the corner colours produces solid white sides and an excessively dark central panel.

**[ours]** Read the four BGRA integer colours at data words 21, 25, 29 and 33. Carry their
RGBA values on the neutral draw node, clockwise from top left. Absent tails retain the ordinary
uniform-colour behavior. Only type 9 objects use this tail; other image tails have other meanings.
Runtime script changes are read from current object data, rather than a cached package constant.

The presenter multiplies each vertex tint by the inherited object tint, then premultiplies
RGB by alpha for the existing UI blend path. Signed sizes mirror geometry and its colours together.
Repeated texture tiles sample the original quad's triangle interpolation; every tile must not
restart the gradient. The shared image path handles this; no pause-specific dark overlay is added.

Tests cover real installed pause artwork, inherited opacity, transparent edges, mirrored geometry,
regular image behavior and per-vertex mesh output. GPU and matched reference evidence are separate.
