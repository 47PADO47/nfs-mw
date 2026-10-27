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
and `h` halved *n* times and never below 1. A 1024 x 1024 BC1 texture with 11 mips is 699,064 bytes of pixels; the graphics block is that rounded up to a
multiple of 128 bytes (699,136), see below. The 53
textures of `VEHICLETEX.BNDL` are 512 to 2048 pixels on a side; there are no sRGB format numbers, so diffuse and data
maps are told apart by the sampler that uses them, not by the format.

### Padding of the graphics block **[verified]**

The graphics block of a texture is the pixels rounded **up to a multiple of 128 bytes**, the padding being zeros at
the end: an 8 x 8 BC1 texture with 4 mips is 56 bytes of pixels in a 128-byte block, a 1024 x 1024 BC3 with 11 mips is
1,398,128 bytes in 1,398,144. This holds for all 53 textures of `VEHICLETEX.BNDL` (the install test asserts it) and
for 82 BC1/BC3 textures read with a Python probe from that bundle, `GLOBALTEXTUREDICTIONARY.BNDL` and three `MAPS`
bundles: 8 fit exactly, 74 are padded, none is longer or shorter. Seven textures in uncompressed formats (six RGBA8, one
R8) were not checked. Use `Texture::slice_size()` for the pixel bytes, not the block's length.

### Exporting a texture as `.dds` **[verified]**

The pixels are already in `.dds` order (the mips of a slice largest first, slices one after the other, tightly packed),
so a `.dds` file is a header plus the pixel bytes. `Texture::to_dds()` writes one; the layout is Microsoft's public DDS
format, the numbers below are what it writes:

| Field | Value |
|---|---|
| magic, size | `DDS `, 124 |
| flags | CAPS, HEIGHT, WIDTH, PIXELFORMAT (0x1007), plus MIPMAPCOUNT (0x20000) when there is more than one mip, plus LINEARSIZE (0x80000) for block formats or PITCH (0x8) for uncompressed ones |
| height, width | from the texture header (the header is written height first, then width) |
| pitch or linear size | bytes of the top mip for block formats (`ceil(w/4) * ceil(h/4) * block bytes`), the row pitch for uncompressed |
| mip count | the texture's, at offset 0x1C |
| pixel format | size 32, flags FOURCC (0x4), code `DXT1`, `DXT3` or `DXT5` for DXGI 71, 74 and 77 of a single slice; `DX10` for everything else |
| caps | TEXTURE (0x1000), plus COMPLEX (0x8) and MIPMAP (0x400000) when there is more than one mip |
| DX10 extension (20 bytes) | DXGI format (the texture's own number, so sRGB variants and BC4/BC5/BC7 survive), dimension 3 (2D), misc 0, array size, alpha mode 0 |

Volume textures (depth above 1) are refused; arrays are written as a 2D array, and a cube map would show as six slices.
The dat-to-dds tool (below) writes the legacy header only and only for BC1 and BC3; the export here covers every
format `PixelFormat` names. All 53 shared car textures export with the expected size (`128 + pixel bytes`) and header
fields (`dds_tests::the_shared_car_textures_export`, run with `CHAMELEON_GAME_DIR`).

### What the community tool does **[community]**

YzENT's [Chameleon-dat-to-dds](https://github.com/YzENT/Chameleon-dat-to-dds) (no license, so only its facts are used)
takes the `.dat` files of the bundle unpacker: for a texture the main block `<id>.dat` is the header and block 1
`<id>_texture.dat` the pixels (the unpacker's name for block 1 of a PC texture). It builds a `.dds` from three header
fields and a fixed template. The offsets it reads, per game:

| Game | Format | Width | Height | Mips | Format codes it accepts |
|---|---|---|---|---|---|
| Most Wanted 2012 (32-bit) | byte at 0x1C | u16 at 0x24 | u16 at 0x26 | byte at 0x2D | 0x47 (71, BC1) and 0x4D (77, BC3) |
| Hot Pursuit 2010 | u32 at 0x0C | u16 at 0x12 | u16 at 0x10 | byte at 0x19 | the 4 bytes are copied as the FourCC, so they must already read `DXT1`/`DXT5` |
| Hot Pursuit Remastered (64-bit) | byte at 0x2C | u16 at 0x36 | u16 at 0x34 | byte at 0x3D | 0x47 and 0x4D |

The Most Wanted row matches this doc's texture header (format at 0x1C, width 0x24, height 0x26, mips 0x2D) **[verified]**.
The Hot Pursuit rows could not be checked (no such install). In the tool's Hot Pursuit code the value at the *first*
offset goes into the `.dds` height slot, the opposite order of its Most Wanted code: either those games store height
first, or the tool is only right for square textures **[unconfirmed]**. Its template is a 128-byte header with flags
0x1007 (no MIPMAPCOUNT or LINEARSIZE flag although the count is written) and caps 0x1002; readers that ignore the flags
open it. Its source says RGBA and BGRA textures are not supported, and it skips anything that is not DXT1/DXT5. It pairs
headers and pixel files by sorted directory order, which only works when every texture has both files.

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
