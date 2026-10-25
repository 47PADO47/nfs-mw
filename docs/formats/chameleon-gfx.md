# Chameleon graphics resources

How the Chameleon engine ([chameleon-bundle.md](chameleon-bundle.md)) stores the things a car is drawn from: textures,
vertex layouts, renderables (meshes), materials, shaders and models. Reader:
[`libs/chameleon-gfx`](../../libs/chameleon-gfx). Everything below was measured on the NFS: Most Wanted (2012) install
and checked by loading all 93 high-detail cars **[verified]** unless tagged. Offsets are in the resource's main block
(block 0) unless they say "graphics block" (block 1).

## Texture (type 0x01)

Main block, 0x30 bytes; the graphics block holds the pixels: all mips of all slices, largest mip first, each mip tightly
packed (no row padding).

| Offset | Type | Field |
|---|---|---|
| 0x1C | u32 | DXGI format: 71 = BC1, 77 = BC3 in the car textures (BC2 74, BC4 80, BC5 83, BC7 98 and the `_SRGB` twins are decoded by name only) |
| 0x24 | u16, u16 | width, height |
| 0x28 | u16, u16 | depth, array size (1, 1 for car textures) |
| 0x2D | u8 | mip levels (1024 x 1024 has 11) |

The size of mip *n* is `ceil(w/4) * ceil(h/4) * 8` bytes for BC1 and BC4, `* 16` for the other block formats, with `w`
and `h` halved *n* times and never below 1. A 1024 x 1024 BC1 texture with 11 mips is 699,136 bytes, as stored. The 53
textures of `VEHICLETEX.BNDL` are 512 to 2048 pixels on a side; there are no sRGB format numbers, so diffuse and data
maps are told apart by the sampler that uses them, not by the format.

## VertexDescriptor (type 0x03)

A vertex layout. Header u32 at 0x0C is the element count; elements are 0x14 bytes from 0x10:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u32 | semantic: usage in the low byte, an index in the rest (0x106 is usage 6, index 1) |
| +0x04 | u32 | DXGI format |
| +0x08 | u32 | byte offset in the vertex |
| +0x0C | u32 | slot (always 0) |
| +0x10 | u32 | vertex stride |

The car layouts (strides 32 to 52) use these usages:

| Usage | Format | Meaning |
|---|---|---|
| 1 | 13 (R16G16B16A16_SNORM) | position: x, y, z times **10 metres**; w is 1 **[verified]** |
| 4 | 28 (R8G8B8A8_UNORM) | vertex colour (0xB2B2B2FF on the parts looked at) |
| 5 | 34 (R16G16_FLOAT) | first texture coordinate |
| 6, 7, 8 | 34 | further texture coordinates (index 1, 2, 3) |
| 9, 10 | 13 | a tangent frame as a unit quaternion (x, y, z, w) **[unconfirmed]: which axis is the normal** |
| 13 | 30 (R10G10B10A2_UINT) | not a normal: it reads as `0x1800` on undamaged parts; damage data **[unconfirmed]** |
| 14 | 28 | damage data **[unconfirmed]** |
| 3, 15 | 6 (R32G32B32_FLOAT) | two float3 on glass; meaning unknown |

The position scale of 10 is exact: for every renderable of the Porsche 911 the decoded vertices are at most the stated
bounding-sphere radius from its centre at that scale, within 0.1 %. Normals are not decoded; the converter derives
them from the triangles. The descriptor is not named by the mesh: a mesh's material names a shader, and the shader's
imports include its vertex descriptor.

## Renderable (type 0x05)

A set of meshes drawn together. Main block, with the vertex and index data in the graphics block.

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32[4] | bounding sphere: centre x, y, z and radius, metres |
| 0x10 | u16 | version: **12** |
| 0x12 | u16 | mesh count |
| 0x14 | u32 | offset of the mesh table: one u32 offset per mesh |

A mesh is 0x60 bytes at its table offset:

| Offset | Type | Field |
|---|---|---|
| +0x10 | u32 | primitive: 4 = triangle list, 5 = triangle strip with `0xFFFF` restarts |
| +0x1C | u32 | index count |
| +0x20 | import | the material (a pointer slot, filled by the import table) |
| +0x28 | u32 | offset of the index buffer record |
| +0x2C | u32 | offset of the vertex buffer record |

Buffer records are six u32: `0, 1, format, offset in the graphics block, size, stride`. For indices the stride at +0x14
is 2 (u16 indices) in every car; for vertices the size is the buffer size **rounded up to the loader's alignment**, so
the vertex count is `floor(size / stride)` and the rest is padding (305,328 bytes of 44-byte vertices is 6,939 vertices).
The index buffer is padded to 0x80 too.

Winding is counter-clockwise outward in the engine's right-handed space: the signed volume of the Porsche body is
+4.2 m^3 and of a tyre +0.066 m^3 **[verified]**. The 2012 axes are **x left, y up, z forward**.

## Material (type 0x02)

| Offset | Type | Field |
|---|---|---|
| 0x08 | import | the shader (type 0x53, in `SHADERS.BNDL`) |
| 0x18 | u32 | offset of the constants |
| 0x20 | u32 | number of textures |
| 0x24 | u32 | end of the constants |
| 0x2C | u32 | offset of the texture pointer slots: one import per texture, 4 bytes apart (the sampler states follow) |

The constants are 16-byte slots holding the shader's material parameters in name order
(`PbrMaterialClearcoatFresnel`, `PbrMaterialClearcoatSpecular`, `PbrMaterialDiffuseColour`, ...). Some parameters are a
single float padded to a slot, so a slot is not found by counting names; the diffuse colour is the first slot whose
alpha is exactly 1 and whose red, green and blue are within 0 to 1. That found the intended slot on the three materials
checked in detail (a paint, a plain body and a lamp). Colours are linear.

## Shader (type 0x53)

Only a little of it is read. It imports the vertex descriptor and several compiled programs, and its main block holds
NUL-terminated names:

- The **technique** name, the last capitalised name with an underscore: `VehicleNFS13_BodyPaint`,
  `VehicleNFS13_Body_Textured_NormalMap_NoDamage`, `VehicleNFS13_Glass`, `VehicleNFS13_Wheel_Tyre_Textured_Normalmap_Blurred`,
  `VehicleNFS13_Refraction`, `..._Alpha1bit_...` (alpha-tested). The technique tells what a surface is.
- The **sampler table**: consecutive u32 words that are the offsets of sampler names (`NormalTextureSampler`,
  `DiffuseTextureSampler`, `EffectsTextureSampler`, `SpecularTextureSampler`, ...). The first run that does not name
  the engine's own samplers (`TonemapSampler`, `cubeReflectionSampler`, `EnvironmentSpecularTextureSampler`,
  `AmbientSpecSampler`, `cubeIrradianceSampler`) lists the samplers in **the order of the material's textures**. The
  shader `Body_Textured_NormalMap_NoDamage` lists Normal, Diffuse, Effects, Specular, and its material's four textures
  are, in that order, a normal map, a grey underbody diffuse, the shared 2048 x 2048 effects map and a specular map
  (looked at as images) **[verified]**. So the diffuse texture is `textures[index of "DiffuseTextureSampler"]`.

## Model (type 0x51)

A level-of-detail set. Offset 0x00 is the offset of the renderable pointer slots, u16 at 0x14 the number of levels,
imports one renderable per level (most detailed first). The float at `u32 at 0x04 + 4` onwards are the switch
distances, as squared metres (40,000 = 200 m).

A car body has three levels (the Porsche: 17, 10 and 1 meshes; 1.28 MB, 53 KB and 38 KB of graphics data), a wheel
part two or three.
