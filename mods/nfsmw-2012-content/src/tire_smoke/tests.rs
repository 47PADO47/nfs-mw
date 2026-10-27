use blackbox_modding::content::parse_tire_smoke_profile;
use chameleon_effects::{Task, ops};

use super::*;

fn picture(width: u32, height: u32, shade: u8) -> Picture {
    Picture { width, height, rgba: vec![shade; (width * height * 4) as usize] }
}

fn block(size: usize, floats: &[(usize, f32)]) -> Vec<u8> {
    let mut bytes = vec![0u8; size];
    for (at, value) in floats {
        bytes[*at..*at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn task(op: u8, block: Vec<u8>) -> Task {
    Task { op, block }
}

/// The numbers a test emitter is made of.
#[derive(Clone, Copy)]
struct Spec {
    life: (f32, f32),
    radius: (f32, f32),
    /// The peak, and the radius factor at birth, at the peak and at the end.
    curve: [f32; 4],
    /// The rate at full smoke, and for each m/s of speed, and the speed range of a new particle.
    rate: (f32, f32),
    speed: (f32, f32),
    drag: f32,
    inherit: (f32, f32),
}

impl Spec {
    fn cloud() -> Spec {
        Spec {
            life: (0.75, 1.0),
            radius: (1.0, 1.25),
            curve: [0.15, 0.25, 0.75, 1.0],
            rate: (18.0, 2.0),
            speed: (0.125, 0.5),
            drag: 0.025,
            inherit: (1.0, 1.0),
        }
    }

    fn emitter(&self, picture: Picture) -> vfx::Emitter {
        let [peak, at_start, at_peak, at_end] = self.curve;
        let mut gradient = block(0xA4, &[(0x20, 0.5), (0x24, 0.5), (0x28, 0.6), (0x2C, 1.0)]);
        gradient[0x90..0x95].copy_from_slice(&[0, 80, 60, 0, 0]);
        gradient[0x95..0x9A].copy_from_slice(&[0, 22, 123, 254, 255]);
        gradient[0x9C..0xA0].copy_from_slice(&1.0f32.to_le_bytes());
        gradient[0xA0..0xA4].copy_from_slice(&1.0f32.to_le_bytes());
        let wheel = block(
            0x7C,
            &[
                (0x04, f32::MAX),
                (0x10, self.speed.0),
                (0x14, self.speed.1),
                (0x6C, 0.0001),
                (0x70, self.rate.1),
                (0x74, self.rate.0),
            ],
        );
        let tasks = vec![
            task(ops::EMITTER_WHEEL, wheel),
            task(ops::INITIALISE_AGE_AND_LIFETIME, block(0x20, &[(0x0C, self.life.0), (0x10, self.life.1)])),
            task(ops::INITIALISE_RADIUS, block(0x14, &[(0x08, self.radius.0), (0x0C, self.radius.1)])),
            task(ops::INITIALISE_VELOCITY_INHERITANCE, block(8, &[(0, self.inherit.0), (4, self.inherit.1)])),
            task(ops::INITIALISE_2D_ROTATION, block(0x18, &[(0, 0.0), (4, 1.0), (0x14, -0.25)])),
            task(ops::UPDATE_POS_VEL_WITH_BOUNCE_PLANE, block(0x40, &[(0x34, self.drag)])),
            task(
                ops::UPDATE_PARAMETRIC_RADIUS,
                block(0x28, &[(0x14, peak), (0x18, at_start), (0x1C, at_peak), (0x20, at_end)]),
            ),
            task(ops::PARAMETRIC_SET_COLOUR_GRADIENT, gradient),
            task(ops::PARAMETRIC_SET_SUB_UV, Vec::new()),
            task(ops::RENDER_SPRITE_SUB_UV, Vec::new()),
        ];
        vfx::Emitter {
            picture,
            flipbook: true,
            fx: chameleon_effects::Emitter { flags: 0, tasks, stages: Default::default() },
        }
    }
}

fn profile(set: &SmokeSet) -> blackbox_modding::content::TireSmokeProfile {
    let text = profile_text(set);
    parse_tire_smoke_profile(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn a_cloud_takes_its_numbers_from_the_emitter_and_the_profile_is_accepted_by_the_game_parser() {
    let set = build(&[Spec::cloud().emitter(picture(512, 512, 1))]).unwrap();
    let layer = profile(&set).layers.remove(0);
    assert_eq!(layer.texture, "TIRESMOKE/MW2012/PUFF.PNG");
    assert!(!layer.additive, "smoke blends, it does not add");
    assert_eq!((layer.grid, layer.fps, layer.random_start), (2, 0, true), "a random cell of a 2 x 2 sheet");
    assert!((layer.rate - 18.0 * RATE_SCALE).abs() < 1e-4, "{layer:?}");
    assert!((layer.rate_per_speed - 2.0 * RATE_SCALE).abs() < 1e-4, "{layer:?}");
    assert_eq!(layer.life, 1.0);
    assert!((layer.life_variance - 0.25).abs() < 1e-6, "{layer:?}");
    assert!((layer.speed - 0.3125).abs() < 1e-5 && (layer.speed_variance - 0.1875).abs() < 1e-5);
    assert!((layer.damping - 0.025).abs() < 1e-6, "the bounce plane's drag");
    assert_eq!((layer.inherit, layer.inherit_variance), (1.0, 0.0));
    assert_eq!((layer.lift, layer.spread), (RISE, SPREAD));
    assert_eq!(layer.angle_range, 360.0, "a whole turn of starting rotation");
    assert_eq!(layer.spin, 90.0, "a quarter turn a second");
}

#[test]
fn the_size_and_alpha_follow_the_curves_at_the_keys() {
    let set = build(&[Spec::cloud().emitter(picture(512, 512, 1))]).unwrap();
    let layer = profile(&set).layers.remove(0);
    assert_eq!(layer.keys[0], 0.0);
    // Radius 1.125 on average, a sprite twice that wide; the factor goes 0.25 at birth, 0.75 at the peak (0.15).
    assert!((layer.size[0] - 2.0 * 1.125 * 0.25).abs() < 0.01, "{:?}", layer.size);
    let peak = layer.keys.iter().position(|k| (*k - 0.15).abs() < 0.01).expect("a key at the peak");
    assert!((layer.size[peak] - 2.0 * 1.125 * 0.75).abs() < 0.01);
    assert_eq!(layer.colors[0][3], 0, "the alpha curve starts at 0");
    assert!(layer.colors.iter().any(|c| c[3] > 70), "and rises to its height");
    assert_eq!(layer.colors.last().unwrap()[3], 0, "and the smoke is gone at the end");
}

#[test]
fn a_sprite_is_never_wider_than_the_widest() {
    let huge = Spec { radius: (3.0, 3.0), curve: [0.3, 0.2, 0.75, 2.3], ..Spec::cloud() };
    let set = build(&[huge.emitter(picture(512, 512, 1))]).unwrap();
    assert!(profile(&set).layers[0].size.iter().all(|w| *w <= WIDEST + 1e-3));
}

#[test]
fn a_wide_sheet_is_written_as_a_square_one_with_its_rows_twice() {
    let wide = picture(8, 4, 7);
    let set = build(&[Spec::cloud().emitter(wide.clone())]).unwrap();
    let layer = profile(&set).layers.remove(0);
    assert_eq!((layer.texture.as_str(), layer.grid), ("TIRESMOKE/MW2012/SHEET.PNG", 4));
    let (name, written) = &set.pictures[0];
    assert_eq!(name, "SHEET.PNG");
    assert_eq!((written.width, written.height), (8, 8));
    assert_eq!(written.rgba.len(), 8 * 8 * 4);
    assert_eq!(&written.rgba[..wide.rgba.len()], &wide.rgba[..]);
    assert_eq!(&written.rgba[wide.rgba.len()..], &wide.rgba[..], "the second half is the first again");
}

#[test]
fn grit_and_repeats_are_left_out() {
    let grit = Spec { radius: (0.1, 0.125), ..Spec::cloud() };
    let cloud = Spec::cloud();
    let other = Spec { life: (3.0, 4.0), ..Spec::cloud() };
    let set = build(&[
        grit.emitter(picture(512, 256, 1)),
        cloud.emitter(picture(512, 512, 1)),
        cloud.emitter(picture(512, 512, 1)),
        other.emitter(picture(512, 512, 1)),
    ])
    .unwrap();
    assert_eq!(profile(&set).layers.len(), 2, "the grit is dropped, and the second cloud is the first again");
    assert_eq!(set.pictures.len(), 1, "one picture for both clouds");
}

#[test]
fn an_emitter_without_a_wheel_spawner_makes_no_layer() {
    let mut emitter = Spec::cloud().emitter(picture(512, 512, 1));
    emitter.fx.tasks.retain(|task| task.op != ops::EMITTER_WHEEL);
    assert!(build(&[emitter]).is_none());
    assert!(build(&[]).is_none());
}

#[test]
fn different_pictures_get_different_files() {
    let set = build(&[
        Spec::cloud().emitter(picture(512, 512, 1)),
        Spec { life: (3.0, 4.0), ..Spec::cloud() }.emitter(picture(512, 512, 2)),
    ])
    .unwrap();
    let names: Vec<&str> = set.pictures.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["PUFF.PNG", "PUFF2.PNG"]);
}

#[test]
fn the_files_are_written_under_the_variants_folder() {
    let set = build(&[Spec::cloud().emitter(picture(8, 8, 9))]).unwrap();
    let out = std::env::temp_dir().join(format!("nfsmw-tire-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    write(&out, &set).unwrap();
    let folder = out.join("files/TIRESMOKE/MW2012");
    assert!(folder.join("PUFF.PNG").is_file());
    let text = std::fs::read_to_string(folder.join("profile.toml")).unwrap();
    assert!(text.starts_with("format = 1\n") && text.contains("texture = \"TIRESMOKE/MW2012/PUFF.PNG\""));
    assert!(parse_tire_smoke_profile(&text).is_ok());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_has_a_tarmac_tire_smoke_that_the_game_accepts() {
    let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = Source::open(std::path::Path::new(&root)).unwrap();
    let set = find(&source).expect("the install's tire smoke");
    // `TIRE_SMOKE_OUT=<folder>` keeps what was made, to look at.
    if let Some(out) = std::env::var_os("TIRE_SMOKE_OUT") {
        write(std::path::Path::new(&out), &set).unwrap();
    }
    let parsed = profile(&set);
    assert!(parsed.layers.len() >= 3, "{} layers", parsed.layers.len());
    assert!(set.pictures.iter().all(|(_, p)| p.width == p.height), "every sheet is square");
    assert!(parsed.layers.iter().all(|l| l.grid >= 2));
}
