use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A mod folder of its own with one manufacturer and its logo, a 4 by 2 picture of one colour.
fn mod_with(name: &str, overriding: bool, logo: Option<&[u8]>) -> ModDir {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "nfsmw-makers-{}-{}",
        std::process::id(),
        CALLS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(dir.join("manufacturers")).unwrap();
    std::fs::create_dir_all(dir.join("files/UI")).unwrap();
    let manifest = format!("name = \"{name}\"\nlogo = \"UI/LOGO.PNG\"\noverride = {overriding}\n");
    std::fs::write(dir.join("manufacturers/maker.toml"), manifest).unwrap();
    if let Some(bytes) = logo {
        std::fs::write(dir.join("files/UI/LOGO.PNG"), bytes).unwrap();
    }
    ModDir { id: format!("test.{name}"), dir, priority: 0 }
}

fn png(color: png::ColorType, pixel: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 4, 2);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixel.repeat(8)).unwrap();
    writer.finish().unwrap();
    out
}

#[test]
fn a_manufacturer_with_its_logo_is_published_and_found_by_its_brand() {
    let m = mod_with("Zyx Motors", false, Some(&png(png::ColorType::Rgba, &[1, 2, 3, 4])));
    assert_eq!(register(std::slice::from_ref(&m)), 1);
    let found = find("zyx-motors", false).expect("found ignoring case and punctuation");
    assert_eq!((found.width, found.height), (4, 2));
    assert_eq!(&found.rgba[..4], &[1, 2, 3, 4]);
    assert!(find("Zyx Motors", true).is_none(), "an override is another entry");
    assert_eq!(register(std::slice::from_ref(&m)), 0, "a repeated call adds nothing");
}

#[test]
fn an_override_is_kept_apart_from_the_plain_logo() {
    let plain = mod_with("Qwv", false, Some(&png(png::ColorType::Rgb, &[9, 9, 9])));
    let over = mod_with("Qwv", true, Some(&png(png::ColorType::Grayscale, &[7])));
    assert_eq!(register(&[plain, over]), 2);
    assert_eq!(&find("QWV", false).unwrap().rgba[..4], &[9, 9, 9, 255]);
    assert_eq!(&find("QWV", true).unwrap().rgba[..4], &[7, 7, 7, 255]);
}

#[test]
fn a_manufacturer_with_a_missing_or_bad_logo_is_left_out() {
    assert_eq!(register(&[mod_with("Nologo", false, None)]), 0);
    assert_eq!(register(&[mod_with("Badlogo", false, Some(b"not a png"))]), 0);
    assert!(find("Nologo", false).is_none() && find("Badlogo", false).is_none());
}
