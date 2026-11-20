use super::*;
use crate::app::test_backend::NullBackend;
use blackbox_modding::content::parse_sky_profile;

const PROFILE: &str = r#"
format = 1
exposure = 0.5
wind = 0.1

[[dome]]
name = "night"
file = "SKY/TEST/NIGHT.DDS"

[[dome]]
name = "day"
file = "SKY/TEST/DAY.DDS"
glow = 0.75

[[weather]]
max_cover = 1.0
gain = 2.0
[[weather.key]]
hour = 0.0
dome = "night"
tint = [0.2, 0.2, 0.2]
[[weather.key]]
hour = 6.0
dome = "day"
tint = [1.0, 0.5, 0.25]
"#;

/// A DDS file of one DXT5 block (4 by 4 pixels).
fn dds() -> Vec<u8> {
    let mut bytes = vec![0u8; 128];
    bytes[..4].copy_from_slice(b"DDS ");
    let words = [(4, 124u32), (8, 0x1007), (12, 4), (16, 4), (28, 1), (76, 32), (80, 4), (108, 0x1000)];
    for (at, word) in words {
        bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes[84..88].copy_from_slice(b"DXT5");
    bytes.extend_from_slice(&[0u8; 16]);
    bytes
}

fn install(files: &[&str]) -> (GameDir, std::path::PathBuf) {
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nfsmw-sky-panorama-{}-{count}", std::process::id()));
    let folder = root.join("SKY/TEST");
    std::fs::create_dir_all(&folder).unwrap();
    for file in files {
        std::fs::write(folder.join(file), dds()).unwrap();
    }
    (GameDir::open(&root).unwrap(), root)
}

fn renderer() -> NullBackend {
    NullBackend::new(crate::settings::test_caps::native())
}

#[test]
fn without_a_profile_there_is_no_panorama() {
    let (dir, root) = install(&[]);
    let mut sky = PanoramaSky::default();
    assert!(!sky.active());
    assert!(sky.frame(&mut renderer(), &dir, (12.0, 0.0), 0.0, 0.0).is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_state_blends_the_keys_turns_the_glow_under_the_sun_and_applies_the_exposure() {
    let (dir, root) = install(&["NIGHT.DDS", "DAY.DDS"]);
    let mut renderer = renderer();
    let mut sky = PanoramaSky::default();
    sky.set_profile(Some(Arc::new(parse_sky_profile(PROFILE).unwrap())), &mut renderer);
    assert!(sky.active());
    let state = sky.frame(&mut renderer, &dir, (3.0, 0.5), 0.25, 10.0).expect("both domes load");
    assert!((state.blend - 0.5).abs() < 1e-6, "{state:?}");
    assert!((state.rotation[0] - 1.0).abs() < 1e-6, "a dome with no glow is carried by the wind: 10 * 0.1");
    assert!((state.rotation[1] - 0.5).abs() < 1e-6, "the glow at 0.75 is turned to the sun at 0.25");
    assert_eq!(state.tint[0], [0.1, 0.1, 0.1]);
    assert_eq!(state.tint[1], [0.5, 0.25, 0.125]);
    assert_eq!(state.gain, 2.0);
    sky.release(&mut renderer);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_dome_that_cannot_be_read_leaves_the_procedural_sky_and_is_not_asked_for_again() {
    let (dir, root) = install(&["NIGHT.DDS"]);
    let mut renderer = renderer();
    let mut sky = PanoramaSky::default();
    sky.set_profile(Some(Arc::new(parse_sky_profile(PROFILE).unwrap())), &mut renderer);
    assert!(sky.frame(&mut renderer, &dir, (3.0, 0.5), 0.0, 0.0).is_none(), "DAY.DDS is missing");
    assert_eq!(sky.domes[1], Picture::Failed);
    assert!(matches!(sky.domes[0], Picture::Loaded(_)), "the dome that read stays");
    std::fs::write(root.join("SKY/TEST/DAY.DDS"), dds()).unwrap();
    assert!(sky.frame(&mut renderer, &dir, (3.0, 0.5), 0.0, 0.0).is_none(), "a failure is kept for this profile");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_profile_frees_the_old_pictures_and_the_same_one_changes_nothing() {
    let (dir, root) = install(&["NIGHT.DDS", "DAY.DDS"]);
    let mut renderer = renderer();
    let mut sky = PanoramaSky::default();
    let profile = Arc::new(parse_sky_profile(PROFILE).unwrap());
    sky.set_profile(Some(profile.clone()), &mut renderer);
    sky.frame(&mut renderer, &dir, (3.0, 0.5), 0.0, 0.0).unwrap();
    sky.set_profile(Some(profile), &mut renderer);
    assert!(matches!(sky.domes[0], Picture::Loaded(_)), "the same profile keeps its pictures");
    sky.set_profile(None, &mut renderer);
    assert!(!sky.active() && sky.domes.is_empty());
    let _ = std::fs::remove_dir_all(root);
}
