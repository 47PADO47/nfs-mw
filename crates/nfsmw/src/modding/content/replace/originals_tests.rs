//! The game's own textures kept when a set replaces them: authored pictures, no game data.

use std::sync::Arc;

use blackbox_modding::content::{TextureState, parse_replace_set};
use blackbox_tpk::{PixelFormat, Texture, TextureSource};

use super::active::{Active, LoadedSet};
use super::originals::*;
use super::textures::TextureSlot;

/// A texture of `side` by `side` with a hash of its own, so tests that share the process-wide ledger do not meet.
fn texture(name: &str, hash: u32, side: u32) -> Texture {
    let blocks = (side / 4) * (side / 4);
    let mut texture =
        TextureSource::new(name, side, side, PixelFormat::Dxt1, 1, vec![7; blocks as usize * 8]).into_texture();
    texture.name_hash = hash;
    texture
}

/// A set that replaces the texture `hash` with `slot`.
pub(crate) fn set_replacing(hash: u32, slot: TextureSlot) -> Active {
    set_replacing_all(vec![(hash, slot)])
}

/// A set that replaces each texture of `pairs` (a hash and the slot that takes its place).
pub(crate) fn set_replacing_all(pairs: Vec<(u32, TextureSlot)>) -> Active {
    let mut text =
        String::from("format = 1\n[set]\nkind = \"road_textures\"\nvalue = \"mw2012\"\nvalue_label = \"x\"\n");
    for (hash, _) in &pairs {
        text.push_str(&format!("\n[[texture]]\ntarget_hash = {hash}\nfile = \"REPLACE/X.dds\"\n"));
    }
    let set = Arc::new(parse_replace_set("roads", &text).unwrap());
    let replaced = pairs.into_iter().map(|(hash, slot)| (hash, Arc::new(slot))).collect();
    Active::build(vec![LoadedSet {
        owner: "mod".into(),
        order: 0,
        set,
        models: Vec::new(),
        added: Vec::new(),
        replaced,
    }])
}

#[test]
fn a_texture_that_a_set_replaces_is_kept_the_first_time() {
    let stock = texture("ROAD", 0x5A5A_0001, 8);
    let picture = texture("OWN", 0x5A5A_0002, 16);
    let active = set_replacing(0x5A5A_0001, TextureSlot::ready("mod", "ROAD", picture, TextureState::Keep));
    assert_eq!(original(0x5A5A_0001), None, "nothing replaced yet");
    let mut textures = vec![stock.clone(), texture("OTHER", 0x5A5A_0003, 8)];
    active.replace_textures(&mut textures);
    assert_eq!(textures[0].width, 16, "replaced");
    assert_eq!(original(0x5A5A_0001).as_deref(), Some(&stock), "the game's own pixels are kept");
    assert_eq!(original(0x5A5A_0003), None, "a texture no set replaces is not kept");
    assert!(original_hashes().contains(&0x5A5A_0001));
    // A second section with the same texture does not change what is kept.
    let mut again = vec![texture("ROAD", 0x5A5A_0001, 8)];
    active.replace_textures(&mut again);
    assert_eq!(original(0x5A5A_0001).as_deref(), Some(&stock));
}

#[test]
fn a_picture_is_made_once_and_can_be_looked_at_without_waiting() {
    let slot = TextureSlot::ready("mod", "ROAD", texture("OWN", 1, 8), TextureState::Keep);
    assert!(slot.made().is_none(), "nothing has made it yet");
    slot.make();
    let made = slot.made().expect("made now").expect("a picture");
    assert_eq!(made.width, 8);
}
