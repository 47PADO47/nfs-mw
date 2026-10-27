# blackbox-solid

Reader for **solids** (models) in `GeometryPack` chunks: header, texture and light-material lists,
position markers, shading groups, every vertex buffer (36/44/60-byte formats decoded to their common attributes) and indices,
with each group's base vertex resolved. Skinned solids (effect 2) also get each vertex's skin (`Solid::skin`), and
`rigid_split` divides a solid into one part per bone. Layouts are chosen by the `SolidInfo` version byte: 0x16
(NFS: Most Wanted, PC) is implemented.

`write_geometry_pack` writes the other way: a `GEOMETRY.BIN` from plain vertex and index lists (`SolidSource`,
`GroupSource`), in the layout of the stock car files, so a converter can build car models. Only the 36-byte vertex
layout is written. `SolidSource::from_solid` turns a parsed solid back into a source.

Spec: `docs/formats/models.md`.

License: MIT OR Apache-2.0.
