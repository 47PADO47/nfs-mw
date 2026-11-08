//! With the install (`NFSMW_GAME_DIR`) that has the CustomHUD plugin and its skins: every skin ini parses, every picture it
//! names decodes, and a frame composes. Nothing of the install is copied into the repository.

use blackbox_gauge_skin::{Animation, Inputs, Pictures, Placement, Skin, compose, dds};
use game_install::GameDir;

use super::detect::{Files, choose, plugin_present};

/// The size of every picture a skin loaded.
struct Sizes(Vec<(String, (u32, u32))>);

impl Pictures for Sizes {
    fn size(&self, file: &str) -> Option<(u32, u32)> {
        let wanted = file.to_ascii_lowercase();
        self.0.iter().find(|(name, _)| *name == wanted).map(|(_, size)| *size)
    }
}

fn install() -> GameDir {
    GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).expect("the install opens")
}

#[test]
#[ignore = "needs the CustomHUD plugin and its skins in the install"]
fn the_installed_plugin_is_found_and_its_general_skin_is_chosen() {
    let dir = install();
    assert!(plugin_present(&dir), "scripts/CustomHud.asi and CustomHud.ini are in the install");
    let source = choose(&dir, "CORVETTE").expect("a skin");
    assert!(dir.exists(&source.ini), "{}", source.ini);
}

#[test]
#[ignore = "needs the CustomHUD plugin and its skins in the install"]
fn every_installed_skin_parses_decodes_and_composes() {
    let dir = install();
    let mut failures = Vec::new();
    let mut skins = 0;
    for folder in dir.subdirectories("scripts/CustomHUD") {
        let base = format!("scripts/CustomHUD/{folder}");
        for file in dir.files_in(&base).into_iter().filter(|name| name.to_ascii_lowercase().ends_with(".ini")) {
            let path = format!("{base}/{file}");
            let text = Files::read(&dir, &path).expect("the ini reads");
            let skin = Skin::parse(&String::from_utf8_lossy(&text));
            assert!(!skin.is_empty(), "{path} enables nothing");
            let mut sizes = Vec::new();
            for used in skin.pictures() {
                let picture = format!("{base}/{}", used.file.replace('\\', "/"));
                let Some(bytes) = Files::read(&dir, &picture) else {
                    // A skin may name a picture it does not ship; the plugin would fail to draw that element too.
                    eprintln!("{picture}: missing");
                    continue;
                };
                match dds::decode(&bytes) {
                    Ok(image) => sizes.push((used.file.to_ascii_lowercase(), (image.width, image.height))),
                    Err(error) => failures.push(format!("{picture}: {error}")),
                }
            }
            let placement = Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 0.0, 0.0);
            let inputs = Inputs {
                rpm: 4200.0,
                red_line: 7500.0,
                speed: 123.0,
                gear: 3,
                nitrous: Some(0.6),
                boost: Some(5.0),
                speedbreaker: 0.5,
                ..Inputs::default()
            };
            let quads = compose(&skin, &inputs, &placement, &Sizes(sizes), &mut Animation::default());
            assert!(!quads.is_empty(), "{path} drew nothing");
            skins += 1;
        }
    }
    assert!(skins > 0, "the install has skins");
    assert!(failures.is_empty(), "pictures that did not load: {failures:#?}");
}
