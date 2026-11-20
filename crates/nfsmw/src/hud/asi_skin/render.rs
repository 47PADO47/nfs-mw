//! Turning a frame of a skin into the UI tree the presenter draws: one image node per quad, the sector-masked pictures built
//! on the CPU and registered as textures.

use std::collections::HashMap;

use blackbox_gauge_skin::{Animation, General, Inputs, Placement, Quad, Sector, compose, mask};
use blackbox_hash::bstring_hash;
use blackbox_ui::{DrawList, NodeKind, UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::load::{Loaded, Picture};
use crate::ui::UiAssets;

/// The depth of every node: they draw in list order.
const Z: f32 = 1.0;
const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
/// A masked picture is rebuilt when its sector's ends move this many degrees.
const MASK_STEP: f32 = 1.5;
/// The longest side of a masked picture: a moving arc is built small, the red zone's numbers once at full size.
const MOVING_SIDE: u32 = 128;
const STATIC_SIDE: u32 = 512;
/// How many masked pictures one skin may make. Past it a masked quad draws unmasked.
const MAX_MASKS: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MaskKey {
    base: u32,
    start: i32,
    end: i32,
    inside: u8,
    outside: u8,
}

/// A loaded skin and what carries from frame to frame: the fades, the blink timer and the masked pictures made so far.
pub(super) struct Overlay {
    loaded: Loaded,
    animation: Animation,
    masks: HashMap<MaskKey, u32>,
}

impl Overlay {
    pub fn new(loaded: Loaded) -> Self {
        Self { loaded, animation: Animation::default(), masks: HashMap::new() }
    }

    /// The plugin settings the skin was chosen with: the scale and the offsets.
    pub fn general(&self) -> &General {
        &self.loaded.source.general
    }

    /// The tree of one frame. An empty tree draws nothing.
    pub fn frame(&mut self, assets: &UiAssets, inputs: &Inputs, placement: &Placement) -> UiTree {
        let quads = compose(&self.loaded.skin, inputs, placement, &self.loaded, &mut self.animation);
        let mut list = DrawList::new();
        list.reserve(quads.len());
        for quad in &quads {
            let Some(base) = self.loaded.picture(&quad.picture, quad.additive) else { continue };
            let key = match quad.sector {
                Some(sector) => masked_key(&mut self.masks, assets, base, quad.additive, sector),
                None => base.key,
            };
            list.push(node(quad, key));
        }
        list.into_tree(|node| node.visible)
    }
}

/// The texture of `base` seen through `sector`: made when the sector's ends, in whole steps, have not been seen before.
fn masked_key(
    masks: &mut HashMap<MaskKey, u32>,
    assets: &UiAssets,
    base: &Picture,
    additive: bool,
    sector: Sector,
) -> u32 {
    let start = mask::quantise(sector.start, MASK_STEP);
    let end = mask::quantise(sector.end, MASK_STEP);
    let key = MaskKey { base: base.key, start, end, inside: sector.inside, outside: sector.outside };
    if let Some(found) = masks.get(&key) {
        return *found;
    }
    if masks.len() >= MAX_MASKS {
        return base.key;
    }
    let side = match sector.fine {
        true => STATIC_SIDE,
        false => MOVING_SIDE,
    };
    let picture = base.image.limited(side);
    let masked = mask::masked(
        &picture,
        mask::dequantise(start, MASK_STEP),
        mask::dequantise(end, MASK_STEP),
        sector.inside,
        sector.outside,
    );
    let hash =
        bstring_hash(&format!("customhud-mask:{:08x}:{start}:{end}:{}:{}", base.key, sector.inside, sector.outside))
            .max(1);
    assets.add_textures([masked.to_texture("customhud-mask", hash, additive)]);
    masks.insert(key, hash);
    hash
}

/// The node of one quad, drawing texture `texture`. The world matrix maps the unit square to the quad, turned about its
/// point when it has a turn (the needle).
pub(super) fn node(quad: &Quad, texture: u32) -> UiNode {
    let scale = Mat4::from_scale(Vec3::new(quad.rect.w, quad.rect.h, 1.0));
    let world = match quad.turn {
        None => Mat4::from_translation(Vec3::new(quad.rect.x, quad.rect.y, Z)) * scale,
        Some(turn) => {
            Mat4::from_translation(Vec3::new(turn.about.0, turn.about.1, Z))
                * Mat4::from_rotation_z(turn.radians)
                * Mat4::from_translation(Vec3::new(quad.rect.x - turn.about.0, quad.rect.y - turn.about.1, 0.0))
                * scale
        }
    };
    let colour = quad.colour.rgba();
    UiNode {
        kind: NodeKind::Image { texture, uv: quad.uv, mask: None, mask_rotation: [0.5, 0.5, 0.0], mask_uv: FULL_UV },
        colour,
        world_colour: colour,
        world,
        position: world.w_axis.truncate(),
        size: Vec3::new(world.x_axis.length(), world.y_axis.length(), 1.0),
        z: Z,
        visible: true,
        ..UiNode::default()
    }
}

#[cfg(test)]
mod tests {
    use blackbox_gauge_skin::{Colour, Rect, Turn};

    use super::*;

    fn quad(turn: Option<Turn>) -> Quad {
        Quad {
            picture: "needle.dds".to_owned(),
            additive: true,
            rect: Rect { x: 10.0, y: 20.0, w: 40.0, h: 20.0 },
            uv: [0.0, 0.0, 0.5, 1.0],
            colour: Colour(0xCC11_2233),
            turn,
            sector: None,
        }
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn an_unturned_quad_fills_its_rectangle() {
        let node = node(&quad(None), 77);
        assert!(close(node.world.transform_point3(Vec3::new(0.0, 0.0, 0.0)), Vec3::new(10.0, 20.0, Z)));
        assert!(close(node.world.transform_point3(Vec3::new(0.5, 0.5, 0.0)), Vec3::new(30.0, 30.0, Z)));
        assert_eq!(node.colour, [0x11, 0x22, 0x33, 0xCC]);
        assert_eq!(node.world_colour, node.colour);
        assert!(node.visible);
        assert_eq!(
            node.kind,
            NodeKind::Image {
                texture: 77,
                uv: [0.0, 0.0, 0.5, 1.0],
                mask: None,
                mask_rotation: [0.5, 0.5, 0.0],
                mask_uv: FULL_UV,
            }
        );
    }

    #[test]
    fn a_turned_quad_keeps_its_pivot_still() {
        // The right end of the quad, (30, 20), is the pivot; turn a quarter circle clockwise on the screen.
        let turn = Turn { radians: std::f32::consts::FRAC_PI_2, about: (30.0, 20.0) };
        let node = node(&quad(Some(turn)), 1);
        let pivot = node.world.transform_point3(Vec3::new(0.5, 0.0, 0.0));
        assert!(close(pivot, Vec3::new(30.0, 20.0, Z)), "the pivot stays where it was: {pivot:?}");
        // The left end, 40 units from the pivot, swings up (y decreases) after a quarter turn clockwise.
        let tip = node.world.transform_point3(Vec3::new(-0.5, 0.0, 0.0));
        assert!(close(tip, Vec3::new(30.0, -20.0, Z)), "the tip points up after 90 degrees: {tip:?}");
    }

    #[test]
    fn a_needle_at_zero_degrees_points_left() {
        let turn = Turn { radians: 0.0, about: (30.0, 20.0) };
        let node = node(&quad(Some(turn)), 1);
        let tip = node.world.transform_point3(Vec3::new(-0.5, 0.0, 0.0));
        assert!(close(tip, Vec3::new(-10.0, 20.0, Z)));
    }
}
