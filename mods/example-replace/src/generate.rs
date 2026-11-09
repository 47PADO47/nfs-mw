//! The files of the example: a lamp model, its texture, a road picture and the two replacement sets that use them. Everything is made
//! here, so the repository holds no model and no picture (and no game data). Change the two names below to replace something else.

use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::{PixelFormat, TextureSource, write_tpk};

/// The game's lamp solids the slim lamp takes the place of (every level of detail is its own solid).
pub const LAMP_TARGETS: &[&str] = &["XT_LAMPPOST_A_00"];
/// The game's texture the checker picture takes the place of.
pub const ROAD_TEXTURE: &str = "ROAD_ASPHALT_01";

/// The name of the new lamp solid and of its texture.
pub const LAMP_MODEL: &str = "EXAMPLE_SLIM_LAMP";
pub const LAMP_TEXTURE: &str = "EXAMPLE_LAMP_GREY";

/// One file the mod writes: its path under the mod's folder and its bytes.
pub struct File {
    pub path: &'static str,
    pub bytes: Vec<u8>,
}

fn vertex(position: [f32; 3], normal: [f32; 3]) -> Vertex {
    Vertex { position, normal, color_bgra: [200; 4], uv: [0.0, 0.0] }
}

/// A box from `low` to `high` as one group of 24 vertices (flat normals), effect 0 (the world shader).
fn cuboid(low: [f32; 3], high: [f32; 3]) -> GroupSource {
    let [x0, y0, z0] = low;
    let [x1, y1, z1] = high;
    let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], [0.0, -1.0, 0.0]),
        ([[x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1]], [0.0, 1.0, 0.0]),
        ([[x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1]], [-1.0, 0.0, 0.0]),
        ([[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]], [1.0, 0.0, 0.0]),
        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], [0.0, 0.0, 1.0]),
        ([[x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0]], [0.0, 0.0, -1.0]),
    ];
    let mut vertices = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    for (corners, normal) in faces {
        let base = vertices.len() as u16;
        vertices.extend(corners.iter().map(|corner| vertex(*corner, normal)));
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    GroupSource { effect_id: 0, flags: 0x4000, ..GroupSource::new(vertices, indices) }
}

/// The geometry pack: a slim pole 6 m tall with an arm and a head, modelled with its foot at the origin.
pub fn lamp_geometry() -> Vec<u8> {
    let pole = cuboid([-0.08, -0.08, 0.0], [0.08, 0.08, 6.0]);
    let arm = cuboid([0.0, -0.05, 5.8], [1.4, 0.05, 5.9]);
    let head = cuboid([1.2, -0.15, 5.7], [1.7, 0.15, 5.8]);
    let mut model = SolidSource::new(LAMP_MODEL, vec![pole, arm, head]);
    model.texture_hashes = vec![bstring_hash(LAMP_TEXTURE)];
    write_geometry_pack(&[model]).expect("the lamp is a valid solid")
}

/// The texture pack: one 8 by 8 DXT1 grey (a DXT1 block of one colour: 0x7BEF both ends, all texels the first colour).
pub fn lamp_textures() -> Vec<u8> {
    let block = [0xEF, 0x7B, 0xEF, 0x7B, 0, 0, 0, 0];
    let data: Vec<u8> = block.iter().copied().cycle().take(32).collect();
    let texture = TextureSource::new(LAMP_TEXTURE, 8, 8, PixelFormat::Dxt1, 1, data);
    write_tpk("EXAMPLE", &[texture]).expect("the texture is valid")
}

/// A 64 by 64 black and white checker as a PNG.
pub fn checker_png() -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 64, 64);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header");
    let mut data = Vec::with_capacity(64 * 64 * 3);
    for y in 0..64u32 {
        for x in 0..64u32 {
            let on = (x / 8 + y / 8) % 2 == 0;
            let value = if on { 230 } else { 40 };
            data.extend([value, value, value]);
        }
    }
    writer.write_image_data(&data).expect("the pixels");
    writer.finish().expect("the PNG");
    out
}

/// The set of the lamps, for the default targets.
pub fn lamp_set() -> String {
    lamp_set_for(LAMP_TARGETS)
}

/// The set of the lamps for `targets`, the game's solids the slim lamp takes the place of.
pub fn lamp_set_for<S: AsRef<str>>(targets: &[S]) -> String {
    let targets: Vec<String> = targets.iter().map(|name| format!("\"{}\"", name.as_ref())).collect();
    format!(
        "format = 1\n\n[set]\nkind = \"street_lamps\"\nkind_label = \"Street lamps\"\nvalue = \"slim\"\nvalue_label = \"Slim lamps\"\nfit = \"stand\"\n\n[files]\ngeometry = \"EXAMPLE/GEOMETRY.BIN\"\ntextures = \"EXAMPLE/TEXTURES.BIN\"\n\n[[model]]\ntargets = [{}]\nmodel = \"{LAMP_MODEL}\"\n",
        targets.join(", ")
    )
}

/// The set of the road texture, for the default target.
pub fn road_set() -> String {
    road_set_for(ROAD_TEXTURE)
}

/// The set of a texture: a PNG, converted to BC1 on the first section that holds `target`.
pub fn road_set_for(target: &str) -> String {
    format!(
        "format = 1\n\n[set]\nkind = \"road_textures\"\nkind_label = \"Road textures\"\nvalue = \"checker\"\nvalue_label = \"Checker asphalt\"\n\n[[texture]]\ntarget = \"{target}\"\nfile = \"EXAMPLE/CHECKER.png\"\nformat = \"bc1\"\n"
    )
}

/// Every file of the mod.
pub fn files() -> Vec<File> {
    files_for(LAMP_TARGETS, ROAD_TEXTURE)
}

/// Every file of the mod, for other targets (the solids of the game to replace with the lamp, the texture to replace with the checker).
pub fn files_for<S: AsRef<str>>(lamps: &[S], road: &str) -> Vec<File> {
    vec![
        File { path: "files/EXAMPLE/GEOMETRY.BIN", bytes: lamp_geometry() },
        File { path: "files/EXAMPLE/TEXTURES.BIN", bytes: lamp_textures() },
        File { path: "files/EXAMPLE/CHECKER.png", bytes: checker_png() },
        File { path: "replace/road-textures.toml", bytes: road_set_for(road).into_bytes() },
        // Written last: the host reads manifests after the mod has loaded, and the files are in place by then.
        File { path: "replace/street-lamps.toml", bytes: lamp_set_for(lamps).into_bytes() },
    ]
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
