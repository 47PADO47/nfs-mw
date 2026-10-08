//! Tests against the user's own install; ignored without `NFSMW_GAME_DIR`.

use std::path::PathBuf;

use blackbox_chunk::find_all;
use blackbox_feng::Package;

const PACKAGE: u32 = 0x0003_0203;
const COMPRESSED: u32 = 0x0003_0210;

fn install() -> Option<PathBuf> {
    std::env::var_os("NFSMW_GAME_DIR").map(PathBuf::from)
}

fn read(rel: &str) -> Vec<u8> {
    let dir = install().unwrap();
    let raw = std::fs::read(dir.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    ea_compress::unwrap(&raw).map(|d| d.into_owned()).unwrap_or(raw)
}

fn packages_in(rel: &str) -> Vec<Package> {
    let data = read(rel);
    let mut out = Vec::new();
    for c in find_all(&data, PACKAGE) {
        out.push(Package::parse(c.payload).unwrap_or_else(|e| panic!("{rel}: {e}")));
    }
    for c in find_all(&data, COMPRESSED) {
        out.push(Package::parse_compressed(c.payload).unwrap_or_else(|e| panic!("{rel}: {e}")).1);
    }
    out
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_package_of_the_install_parses() {
    if install().is_none() {
        return;
    }
    let mut total = 0;
    for rel in [
        "FRONTEND/FrontB.lzc",
        "GLOBAL/InGameB.bun",
        "GLOBAL/INGAMEC.BUN",
        "GLOBAL/GLOBALA.BUN",
        "GLOBAL/GlobalB.lzc",
        "GLOBAL/InGameSplitScreen.bun",
        "GLOBAL/WIDESCREEN_GLOBAL.BUN",
        "GLOBAL/THINSCREEN_GLOBAL.BUN",
    ] {
        let pk = packages_in(rel);
        println!("{rel}: {} packages", pk.len());
        for p in &pk {
            assert!(!p.name.is_empty(), "{rel}: unnamed package");
            assert!(
                p.objects.iter().all(|o| o.parent.is_none_or(|g| p.find_by_guid(g).is_some())),
                "{}: orphan",
                p.name
            );
        }
        total += pk.len();
    }
    assert!(total >= 200, "only {total} packages");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_single_race_hud_has_its_objects() {
    if install().is_none() {
        return;
    }
    let hud = packages_in("GLOBAL/InGameB.bun")
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case("HUD_SingleRace.fng"))
        .unwrap();
    assert_eq!(hud.objects.len(), 372);
    assert_eq!(hud.resources.len(), 69);
    for name in ["SpeedometerGroup", "GaugeCluster", "3rdPersonNeedle", "3rdPersonGear", "SPEED_DIGIT_1"] {
        assert!(hud.find_by_hash(blackbox_feng::fe_hash_upper(name)).is_some(), "{name} is missing");
    }
    let needle = &hud.objects[hud.find_by_hash(blackbox_feng::fe_hash_upper("3rdPersonNeedle")).unwrap()];
    assert_eq!(needle.kind, blackbox_feng::package::ObjectKind::Image);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_game_fonts_parse_and_lay_out_text() {
    if install().is_none() {
        return;
    }
    let mut names = Vec::new();
    for rel in [
        "LANGUAGES/English.bin",
        "FRONTEND/FrontB.lzc",
        "GLOBAL/InGameB.bun",
        "GLOBAL/GLOBALA.BUN",
        "GLOBAL/GlobalB.lzc",
        "GLOBAL/InGameA.bun",
        "GLOBAL/INGAMEC.BUN",
    ] {
        let data = read(rel);
        for c in find_all(&data, 0x0003_0201) {
            let f = blackbox_feng::Font::parse(c.payload).unwrap();
            println!(
                "{rel}: {} / {}: {} glyphs, ascent {} descent {}, v{}",
                f.name,
                f.texture_name,
                f.glyphs().len(),
                f.ascent,
                f.descent,
                f.version
            );
            names.push(f);
        }
    }
    let body = names.iter().find(|f| f.name.eq_ignore_ascii_case("font_mw_body")).expect("font_mw_body");
    let a = body.glyph(b'A' as u16).unwrap();
    assert_eq!((a.width, a.height, a.u, a.v, a.advance_x), (15, 14, 118, 64, 13));
    let layout = body.layout("100 KM/H", blackbox_feng::font::TextStyle::default(), (256, 256));
    assert_eq!(layout.quads.len(), 8);
    assert!(layout.width > 40.0 && layout.width < 200.0, "{}", layout.width);
}
