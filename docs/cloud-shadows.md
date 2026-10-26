# Enhanced cloud shadows

The native renderer can project moving sunlight attenuation from the Enhanced sky
onto world surfaces and vehicles. F8 > Environment > Cloud shadows toggles the
feature independently of geometric sun shadows. It is off by default and requires
Enhanced sky. Clear coverage and low sun use a neutral fallback.

The field is generated from the same procedural density and wind as the sky. A
256-square map covers an 8 km reference plane, with an approximate altitude
correction and a neutral edge fade. It is refreshed in the main view before
reflection captures. Direct sunlight is attenuated while ambient and reflection
terms stay separate. Authored prelit scenery and original car lighting use
limited approximations because their baked components cannot be recovered.

The 8-sample march, map size and strength are initial experimental choices.
Long-drive stability, sun/sky alignment, reflection views, HDR and performance
need playtesting. The field is not a model of ambient sky occlusion or volumetric
shafts. Original sky leaves this enhancement inactive.
