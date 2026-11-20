use super::*;
use crate::grade::lift;

fn level(width: u32, height: u32, texel: [u8; 4]) -> Level {
    Level { width, height, rgba: texel.repeat((width * height) as usize) }
}

#[test]
fn occlusion_darkens_down_to_the_floor_and_no_occlusion_keeps_the_colour() {
    let pbr = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 0] };
    let dark =
        bake_diffuse(level(2, 2, [200, 200, 200, 255]), Some(&pbr), true, Grade { ao_floor: 0.3, ..Grade::NONE }, None);
    // Fully occluded: 0.3 of the linear colour.
    let expected = linear_to_srgb(srgb_to_linear(200) * 0.3);
    assert_eq!(dark.rgba[0], expected);
    assert!(expected < 200 && expected > 60);
    let kept = bake_diffuse(
        level(2, 2, [200, 200, 200, 255]),
        Some(&pbr),
        false,
        Grade { ao_floor: 0.3, ..Grade::NONE },
        None,
    );
    assert_eq!(kept.rgba[0], 200);
    let open = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 255] };
    let lit = bake_diffuse(
        level(2, 2, [200, 200, 200, 255]),
        Some(&open),
        true,
        Grade { ao_floor: 0.3, ..Grade::NONE },
        None,
    );
    assert_eq!(lit.rgba[0], 200, "an unoccluded texel keeps its colour");
}

#[test]
fn the_lift_is_applied_after_the_occlusion_and_leaves_the_alpha() {
    let pbr = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 0] };
    let plain =
        bake_diffuse(level(1, 1, [200, 200, 200, 255]), Some(&pbr), true, Grade { ao_floor: 0.3, ..Grade::NONE }, None);
    let lifted = bake_diffuse(
        level(1, 1, [200, 200, 200, 77]),
        Some(&pbr),
        true,
        Grade { ao_floor: 0.3, gain: 1.35, reflected: 0.0 },
        None,
    );
    assert_eq!(lifted.rgba[0], lift(plain.rgba[0], 1.35));
    assert_eq!(lifted.rgba[3], 255, "no clip value means opaque");
    let no_pictures = bake_diffuse(
        level(1, 1, [100, 100, 100, 255]),
        None,
        false,
        Grade { ao_floor: 0.3, gain: 1.35, reflected: 0.0 },
        None,
    );
    assert_eq!(no_pictures.rgba[0], lift(100, 1.35), "the lift needs no occlusion picture");
}

#[test]
fn a_dielectric_reflects_the_environment_and_a_metal_does_not() {
    let grade = Grade { reflected: 0.5, ..Grade::NONE };
    // Specular 0.5 (F0 = 0.04) reflecting a radiance of 0.5: 0.02 linear, which is sRGB 38 on a black texel.
    let plastic = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 255] };
    let lit = bake_diffuse(level(1, 1, [0, 0, 0, 255]), Some(&plastic), true, grade, None);
    assert!((36..=40).contains(&lit.rgba[0]), "{}", lit.rgba[0]);
    let steel = Picture { width: 1, height: 1, rgba: vec![128, 100, 255, 255] };
    let dark = bake_diffuse(level(1, 1, [0, 0, 0, 255]), Some(&steel), true, grade, None);
    assert_eq!(dark.rgba[0], 0, "a metal takes its reflection from the light material");
    let none = bake_diffuse(level(1, 1, [0, 0, 0, 255]), Some(&plastic), true, Grade::NONE, None);
    assert_eq!(none.rgba[0], 0);
}

#[test]
fn a_saturated_colour_is_not_veiled_by_the_reflection() {
    let grade = Grade { reflected: 0.5, ..Grade::NONE };
    let plastic = Picture { width: 1, height: 1, rgba: vec![128, 100, 0, 255] };
    // The red of the Honda CRF 450 R: no green, so a grey veil would turn it pink.
    let red = bake_diffuse(level(1, 1, [184, 0, 26, 255]), Some(&plastic), true, grade, None);
    let kept = [184u8, 0, 26].iter().zip(&red.rgba[..3]).all(|(&want, &got)| want.abs_diff(got) <= 1);
    assert!(kept, "a fully saturated texel keeps its colour: {:?}", &red.rgba[..3]);
    let grey = bake_diffuse(level(1, 1, [60, 60, 60, 255]), Some(&plastic), true, grade, None);
    assert!(grey.rgba[0] > 60, "a grey texel takes the whole veil: {}", grey.rgba[0]);
}

#[test]
fn the_opacity_is_cut_at_the_clip_value() {
    let mut base = level(2, 1, [10, 20, 30, 0]);
    base.rgba[3] = 200;
    base.rgba[7] = 40; // 40 / 255 is below the clip of 1/3
    let cut = bake_diffuse(base, None, false, Grade { ao_floor: 0.3, ..Grade::NONE }, Some(0.3333));
    assert_eq!((cut.rgba[3], cut.rgba[7]), (255, 0));
    let opaque = bake_diffuse(level(1, 1, [1, 2, 3, 9]), None, false, Grade { ao_floor: 0.3, ..Grade::NONE }, None);
    assert_eq!(opaque.rgba[3], 255, "no clip value means opaque");
}

#[test]
fn srgb_round_trips() {
    for v in [0u8, 1, 17, 128, 200, 255] {
        assert_eq!(linear_to_srgb(srgb_to_linear(v)), v);
    }
}

#[test]
fn the_specular_is_bright_for_smooth_metal_and_dark_for_rough_dielectric() {
    let like = level(1, 1, [0; 4]);
    let base = Picture { width: 1, height: 1, rgba: vec![255, 255, 255, 255] };
    let smooth_metal = Picture { width: 1, height: 1, rgba: vec![128, 0, 255, 255] };
    let rough_plastic = Picture { width: 1, height: 1, rgba: vec![128, 255, 0, 255] };
    assert_eq!(bake_specular(&like, &base, &smooth_metal).rgba[0], 255);
    assert_eq!(bake_specular(&like, &base, &rough_plastic).rgba[0], 0);
}

#[test]
fn a_normal_map_can_have_its_green_flipped() {
    let picture = Picture { width: 1, height: 1, rgba: vec![10, 20, 250, 7] };
    let flipped = bake_normal(picture, 8, true);
    assert_eq!(flipped.rgba, vec![10, 235, 250, 255]);
}

fn write_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(rgba).unwrap();
}

#[test]
fn a_translucent_material_bakes_a_blended_texture_with_the_opacity_in_the_alpha() {
    let dir = std::env::temp_dir().join(format!("ride6-bikes-glass-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Two texels of a 8 x 4 picture: clear on the left half, a red lens on the right.
    let texel = |x: u32| if x < 4 { [255, 255, 255, 0] } else { [173, 0, 5, 36] };
    let rgba: Vec<u8> = (0..32).flat_map(|i| texel(i % 8)).collect();
    write_png(&dir.join("glass_d.png"), 8, 4, &rgba);
    let material = Material {
        name: "mat_glass".into(),
        blend_mode: Some("BLEND_TranslucentGreyTransmittance".into()),
        base_color: Some("glass_d".into()),
        transmittance: Some("glass_d".into()),
        transmittance_max: Some(0.95),
        ..Material::default()
    };
    let mut surfaces = Surfaces::new(TexStore::open(&dir).unwrap(), Options::default());
    let surface = surfaces.surface("CAR", &material, Kind::Body).unwrap();
    assert_eq!(surface.diffuse, "CAR_GLASS_D");
    assert!(surface.normal.is_none() && surface.pbr.is_none());
    assert!((surface.glass.as_ref().unwrap().max - 0.95).abs() < 1e-6);
    let textures = std::mem::take(&mut surfaces.palette).into_textures();
    std::fs::remove_dir_all(&dir).unwrap();
    let [glass] = textures.as_slice() else { panic!("{} textures", textures.len()) };
    assert_eq!(glass.format, PixelFormat::Dxt5);
    assert!(glass.alpha_sorting && glass.alpha_blend == 1);
    assert_eq!(glass.alpha_usage, AlphaUsage::Modulated);
    assert_eq!(glass.flags & TWO_SIDED, 0, "a blended surface is drawn one-sided like the stock windows");
}

#[test]
fn the_size_cap_follows_the_part() {
    assert_eq!(size_cap(Kind::Body, 2048), 2048);
    assert_eq!(size_cap(Kind::Rim, 2048), 1024);
    assert_eq!(size_cap(Kind::Brake, 2048), 512);
    assert_eq!(size_cap(Kind::Brake, 8), 4);
}

#[test]
fn a_saturated_paint_is_lifted_without_changing_its_hue() {
    // The red of the Honda CRF 450 R: lifting each channel alone raised its blue from 26 to 33 (crimson); the hue lift keeps the share.
    let lifted = bake_diffuse(level(1, 1, [184, 0, 26, 255]), None, false, Grade { gain: 1.35, ..Grade::NONE }, None);
    let [r, g, b] = [lifted.rgba[0], lifted.rgba[1], lifted.rgba[2]];
    assert!(r > 184 && g == 0, "{r} {g} {b}");
    assert!((f32::from(b) / f32::from(r) - 26.0 / 184.0).abs() < 0.01, "{r} {b}");
}
