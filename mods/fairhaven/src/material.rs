//! What a 2012 material makes of a surface in this game: the picture it is drawn with and the few things of its technique that
//! the game's own materials can say (a cut-out, two sides). The 2012 shading (lightmaps, reflections, lit windows, the blend of
//! the terrain's layers) is not read: a surface is its diffuse picture (`docs/modding/fairhaven.md`).

use chameleon_gfx::Material;

use crate::install::ShaderInfo;

/// The samplers that hold a surface's colour, in the order they are preferred: the buildings and props, the roads, the first
/// layer of the terrain, the decals.
const DIFFUSE_SAMPLERS: [&str; 6] = [
    "DiffuseTextureSampler",
    "ColourMap_Sampler",
    "Tiling1TextureSampler",
    "Decal_Diffuse_Sampler",
    "Line_Diffuse_Sampler",
    "DetailMap_Diffuse_Sampler",
];

/// How a surface is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    /// The texture resource of its colour, when the material has one.
    pub diffuse: Option<u64>,
    /// The picture's alpha cuts the surface out (the `1Bit` techniques, fences).
    pub cut_out: bool,
    /// Both sides are drawn.
    pub two_sided: bool,
    /// The surface is not drawn: cables and the like that the game's materials cannot show.
    pub hidden: bool,
    /// The surface is water (the sea and the rivers): drawn as one flat colour, since the 2012 water is a normal map and a
    /// reflection.
    pub water: bool,
    /// How many times the picture repeats over the mesh's texture coordinates, across and down: the terrain's tiling ratios of
    /// its first layer (the first two parameters of a `Plot*` material, `docs/formats/chameleon-world.md`), 1 for the rest.
    pub uv_scale: [f32; 2],
}

/// The tiling of a terrain material's first layer: the x of its `Tiling_Ratios_X` parameter and of its `Tiling_Ratios_Y`
/// (slots 0 and 1 of a `Plot*` material, 1 and 2 of a `Terrain*` one, which has the noise ratio first). A value that is not a
/// positive number leaves the coordinates as they are.
fn tiling(material: &Material, technique: &str) -> [f32; 2] {
    let first = match technique {
        t if t.starts_with("Plot") => 0,
        t if t.starts_with("Terrain") => 1,
        _ => return [1.0, 1.0],
    };
    let ratio = |slot: usize| material.float4(slot).map(|v| v[0]).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.0);
    [ratio(first), ratio(first + 1)]
}

/// The surface a material has under the technique and samplers of its shader.
pub fn surface(material: &Material, shader: &ShaderInfo) -> Surface {
    let technique = shader.technique.as_str();
    let diffuse = DIFFUSE_SAMPLERS
        .iter()
        .find_map(|name| shader.samplers.iter().position(|sampler| sampler == name))
        .and_then(|slot| material.textures.get(slot).copied())
        .or_else(|| material.textures.first().copied());
    Surface {
        diffuse,
        uv_scale: tiling(material, technique),
        cut_out: technique.contains("1Bit") || technique.contains("1_Bit") || technique.starts_with("Fence"),
        two_sided: technique.contains("Doublesided"),
        hidden: technique.starts_with("Cable"),
        water: technique.starts_with("Water"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shader(technique: &str, samplers: &[&str]) -> ShaderInfo {
        ShaderInfo {
            technique: technique.to_owned(),
            samplers: samplers.iter().map(|s| (*s).to_owned()).collect(),
            elements: Vec::new(),
        }
    }

    fn material(textures: &[u64]) -> Material {
        Material { shader: 0, textures: textures.to_vec(), constants: Vec::new() }
    }

    #[test]
    fn the_diffuse_picture_is_the_one_the_diffuse_sampler_takes() {
        let info = shader(
            "WorldPBR_Diffuse_Normal_Specular_Reflective_AO_Singlesided",
            &["NormalTextureSampler", "DiffuseTextureSampler", "SpecularTextureSampler"],
        );
        let surface = surface(&material(&[10, 20, 30]), &info);
        assert_eq!(surface.diffuse, Some(20));
        assert!(!surface.cut_out && !surface.two_sided && !surface.hidden && !surface.water);
    }

    #[test]
    fn a_road_uses_its_colour_map_and_a_terrain_its_first_layer() {
        let road = shader("DriveableSurface_Decal", &["Decal_Diffuse_Sampler", "ColourMap_Sampler"]);
        assert_eq!(surface(&material(&[1, 2]), &road).diffuse, Some(2), "the colour map comes before the decal");
        let terrain =
            shader("PlotPBR_AO_Normal", &["MaskTextureSampler", "Tiling3TextureSampler", "Tiling1TextureSampler"]);
        assert_eq!(surface(&material(&[7, 8, 9]), &terrain).diffuse, Some(9));
    }

    #[test]
    fn the_name_of_the_technique_says_cut_outs_two_sides_and_what_is_not_drawn() {
        let tree = surface(
            &material(&[5]),
            &shader("Tree_Translucent_1Bit_Doublesided_InstanceTint", &["DiffuseTextureSampler"]),
        );
        assert!(tree.cut_out && tree.two_sided);
        assert!(surface(&material(&[5]), &shader("Fence_GreyScale_Doublesided", &["DiffuseTextureSampler"])).cut_out);
        assert!(surface(&material(&[]), &shader("Cable_GreyScale_Doublesided", &[])).hidden);
        let water = surface(&material(&[3, 4]), &shader("Water_Proto_Cheap", &["NormalTextureSampler"]));
        assert!(water.water && !water.hidden);
    }

    #[test]
    fn a_terrain_repeats_its_first_layer_by_the_tiling_ratios_and_other_surfaces_do_not() {
        let constants: Vec<u8> =
            [50.0f32, 100.0, 80.0, 0.0, 40.0, 30.0, 20.0, 0.0].iter().flat_map(|v| v.to_le_bytes()).collect();
        let terrain = Material { shader: 0, textures: vec![1], constants: constants.clone() };
        let info = shader("PlotPBR_AO_Normal", &["Tiling1TextureSampler"]);
        assert_eq!(surface(&terrain, &info).uv_scale, [50.0, 40.0]);
        let mountain = shader("TerrainPBR_Normal_Opaque", &["Tiling1TextureSampler"]);
        assert_eq!(
            surface(&terrain, &mountain).uv_scale,
            [40.0, 1.0],
            "slots 1 and 2 hold the ratios (here 40 and the second slot's x of 1)"
        );
        let world = shader("World_Diffuse_Specular_Singlesided", &["DiffuseTextureSampler"]);
        assert_eq!(surface(&terrain, &world).uv_scale, [1.0, 1.0]);
        let empty = Material { shader: 0, textures: vec![1], constants: Vec::new() };
        assert_eq!(surface(&empty, &info).uv_scale, [1.0, 1.0]);
    }

    #[test]
    fn a_shader_with_no_known_sampler_falls_back_to_the_first_texture() {
        let info = shader("Something_New", &["OtherSampler"]);
        assert_eq!(surface(&material(&[42, 43]), &info).diffuse, Some(42));
        assert_eq!(surface(&material(&[]), &info).diffuse, None);
    }
}
