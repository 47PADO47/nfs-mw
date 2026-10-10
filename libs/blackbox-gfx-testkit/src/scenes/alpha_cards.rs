//! Alpha-tested foliage cards, overlapping and crossing, in front of a wall.

use blackbox_gfx::{BlendMode, Instance, Shading};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const CLEAR: [f32; 3] = [0.30, 0.34, 0.40];
const LEAF_TINTS: [[f32; 3]; 3] = [[0.35, 0.85, 0.30], [0.80, 0.85, 0.25], [0.95, 0.55, 0.20]];

pub(super) fn build(cx: &mut Cx) -> Parts {
    let leaf = cx.texture("leaf card", &Image::leaf_card(64), Storage::Rgba8);
    let floor = cx.texture("cards floor", &Image::checker(32, 8, [90, 80, 60], [60, 55, 40]), Storage::Bc1);

    let mut backdrop = MeshBuilder::new();
    backdrop.state(None, BlendMode::Opaque, Shading::Prelit);
    let (dark, light) = (prelit([0.15, 0.30, 0.38], 1.0), prelit([0.55, 0.75, 0.85], 1.0));
    let wall = [
        Vec3::new(-14.0, 16.0, 0.0),
        Vec3::new(14.0, 16.0, 0.0),
        Vec3::new(14.0, 16.0, 11.0),
        Vec3::new(-14.0, 16.0, 11.0),
    ];
    backdrop.quad(wall, [dark, dark, light, light], [1.0, 1.0]);
    backdrop.state(Some(floor), BlendMode::Opaque, Shading::Prelit);
    let ground = [
        Vec3::new(-14.0, -4.0, 0.0),
        Vec3::new(14.0, -4.0, 0.0),
        Vec3::new(14.0, 16.0, 0.0),
        Vec3::new(-14.0, 16.0, 0.0),
    ];
    backdrop.quad(ground, [prelit([1.0; 3], 1.0); 4], [7.0, 3.0]);
    let backdrop = cx.mesh("cards backdrop", &backdrop.finish());

    let mut cards = MeshBuilder::new();
    cards.state(Some(leaf), BlendMode::AlphaTest, Shading::Prelit);
    for k in 0..16 {
        let (row, col) = ((k / 8) as f32, (k % 8) as f32);
        let tint = LEAF_TINTS[k % 3];
        let centre = Vec3::new(-10.5 + col * 3.0 + row * 1.4, 6.0 + row * 3.0 + (col * 0.9).sin(), 0.2 + row * 0.5);
        cards.card(centre, 3.4, 3.4, (col - 3.5) * 0.12, prelit(tint, 1.0));
    }
    // Two cards crossing each other: the alpha test must keep the depth of what survives.
    cards.card(Vec3::new(0.0, 12.0, 0.0), 4.0, 4.0, 0.9, prelit([0.9, 0.9, 0.9], 1.0));
    cards.card(Vec3::new(0.0, 12.0, 0.0), 4.0, 4.0, -0.9, prelit([0.6, 0.9, 1.0], 1.0));
    let cards = cx.mesh("cards leaves", &cards.finish());

    let instances = vec![
        Instance { mesh: backdrop, transform: Mat4::IDENTITY },
        Instance { mesh: cards, transform: Mat4::IDENTITY },
    ];
    let camera = Camera::new(Vec3::new(0.0, -3.0, 3.0), Vec3::new(0.0, 10.0, 2.5));
    Parts::world(camera.frame(cx.aspect, CLEAR, None), instances)
}
