//! A category's replacement hints must leave its native item caption readable.
use super::{Harness, Page, Section, model};
use crate::input::{Bindings, InputDevice};
use blackbox_feng::{NodeKind, font::TextStyle, runtime::pad};

#[test]
#[ignore = "requires installed category packages and fonts"]
fn batch12_category_footer_glyphs_do_not_overlap_the_selected_item_caption() {
    let mut h = Harness::open(Page::Categories { section: Section::Performance, backroom: false }, model());
    for _ in 0..4 {
        for device in [InputDevice::Xbox, InputDevice::Keyboard] {
            let raw = h.screens.trees().pop().unwrap();
            let pages = h.screens.presented(&Bindings::default(), device);
            let tree = &pages.last().unwrap().tree;
            let Some((left, y0, y1)) = crate::frontend::prompt_layout::caption_left(tree, h.screens.assets()) else {
                panic!("caption must be visible")
            };
            let mut checked = 0;
            for n in &tree.nodes[raw.nodes.len()..] {
                if !n.visible {
                    continue;
                }
                let NodeKind::Text { font, justification, leading, max_width } = n.kind else { continue };
                let assets = h.screens.assets();
                let font = assets.font(font).unwrap();
                let layout = font.layout_scaled(
                    n.text.as_deref().unwrap(),
                    TextStyle { justification, leading, max_width },
                    assets.texture_size(assets.resolve(font.texture_hash)).unwrap(),
                    assets.font_scale(font.hash),
                );
                for q in layout.quads {
                    let a = n.world.transform_point3(glam::Vec3::new(q.x0, q.y0, 0.0));
                    let b = n.world.transform_point3(glam::Vec3::new(q.x1, q.y1, 0.0));
                    if b.y >= y0 && a.y <= y1 {
                        assert!(b.x <= left, "hint {:?} overlaps caption at {left}", n.text);
                    }
                }
                checked += 1;
            }
            assert!(checked > 0, "replacement footer must remain visible");
        }
        h.press(pad::RIGHT);
    }
}
