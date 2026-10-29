//! Bind only the images that exist in the rival package. The bio has a background and graffiti; it has no portrait.
//! The texture-ready messages follow the existing helper spec in `career-regions.md` sections 7/8.

use blackbox_feng::fe_hash_upper;

use crate::frontend::logic::Cx;

pub(super) fn bind(cx: &mut Cx, bin: u8) {
    for (name, suffix) in [(0xC1F6_2308, ""), (0xF5A2_A087, "_GRAF"), (0x2CBE_1DD0, "_BG")] {
        let Some(image) = cx.object(name) else { continue };
        let texture = fe_hash_upper(&format!("RIVAL_{bin:02}{suffix}"));
        let loaded = cx.assets.texture(texture).is_some();
        cx.rt.set_hidden(image, !loaded);
        if loaded {
            cx.rt.set_texture(image, texture);
        }
    }
    // The bio's texture-ready handler posts RIVAL_CONTENT_START; that script reveals the background.
    for message in [0xC094_2E85, 0x8C9D_4547, 0xD22B_95D0] {
        cx.rt.post_to_package(cx.package, message);
    }
}
