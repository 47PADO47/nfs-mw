//! Depth-tested lens and camera glare for powered street fixtures.
use blackbox_gfx::{EffectLayer, EffectVertex};
use glam::Vec3;

use super::super::{
    car_light_flares::FlareView,
    vehicle_effects::geometry::{color, glow},
};

/// A lens-shaped source and a camera-facing core remain visible at driving distance.
/// The separate halo is intentionally small and does not change light on nearby surfaces.
/// A lantern (zero lens axes) keeps only the camera-facing glow, and none when its drawn model's
/// own glass emits (non-zero `lantern_z`).
pub(super) fn append(
    out: &mut Vec<EffectVertex>,
    lenses: &mut Vec<EffectVertex>,
    (at, intensity, lens_x, lens_y, lantern_z): (Vec3, f32, Vec3, Vec3, Vec3),
    view: FlareView,
    strength: f32,
) {
    let depth = (at - view.camera).dot(view.forward);
    let gain = intensity * strength;
    if gain <= 0.0 {
        return;
    }
    // Every LOD of this lantern draws emissive glass from its original triangles: no sprite stands in for it.
    if lantern_z.length_squared() > 0.0 {
        return;
    }
    // No panel is invented for a lantern whose glass cannot be identified; its glow below remains.
    if lens_x.length_squared() > 0.0 && lens_y.length_squared() > 0.0 {
        let lens_start = lenses.len();
        let tint = color([1.0, 0.95, 0.82, 1.0], gain);
        EffectLayer::quad(
            lenses,
            [at - lens_x - lens_y, at + lens_x - lens_y, at + lens_x + lens_y, at - lens_x + lens_y],
            tint,
        );
        for vertex in &mut lenses[lens_start..] {
            vertex.detail = [(4.0 * strength).clamp(1.0, 8.0), -1.0];
        }
    }
    if depth <= 0.0 {
        return;
    }
    let focal = (view.height as f32 * 0.5) / (view.fov_degrees.to_radians() * 0.5).tan().max(0.01);
    let core_radius = (depth * 1.2 / focal.max(1.0)).clamp(0.05, 1.2);
    let halo_radius = (core_radius * 3.0).clamp(0.18, 3.5);
    let center = at + (view.camera - at).normalize_or_zero() * 0.08;
    let halo = color([1.0, 0.87, 0.68, 1.0], gain * 0.12);
    glow(out, center, halo_radius, halo, view.camera, view.forward);
    // The lens owns the close-up shape; a small camera-facing core only fills in at distance.
    let distant = ((depth - 25.0) / 75.0).clamp(0.0, 1.0);
    let core = color([1.0, 0.98, 0.92, 1.0], gain * (0.16 + 0.5 * distant));
    let first = out.len();
    glow(out, center, core_radius, core, view.camera, view.forward);
    // HDR scenes can bloom from the bulb; ordinary LDR targets still clamp safely.
    for vertex in &mut out[first..] {
        vertex.detail[0] = (3.0 * strength).clamp(1.0, 6.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_has_a_bright_core_and_switches_off_independently() {
        let view = FlareView { camera: Vec3::ZERO, forward: Vec3::X, fov_degrees: 60.0, height: 1080 };
        let lamp = (Vec3::X * 30.0, 1.0, Vec3::Y * 0.2, Vec3::Z * 0.1, Vec3::ZERO);
        let mut vertices = Vec::new();
        let mut lenses = Vec::new();
        append(&mut vertices, &mut lenses, lamp, view, 1.0);
        assert_eq!(vertices.len(), 12);
        assert_eq!(lenses.len(), 6);
        assert!(vertices[6].color[3] > vertices[0].color[3], "the core must be brighter than the halo");
        assert!(lenses[0].detail[1] < 0.0, "the physical lens uses the lens mask");
        vertices.clear();
        lenses.clear();
        append(&mut vertices, &mut lenses, lamp, view, 0.0);
        assert!(vertices.is_empty());
        assert!(lenses.is_empty());
        append(&mut vertices, &mut lenses, (-lamp.0, lamp.1, lamp.2, lamp.3, lamp.4), view, 1.0);
        assert!(vertices.is_empty());
        assert_eq!(lenses.len(), 6, "offscreen lenses remain available to reflection captures");
    }
    #[test]
    fn lanterns_glow_without_a_fake_lens_unless_their_glass_emits() {
        let view = FlareView { camera: Vec3::ZERO, forward: Vec3::X, fov_degrees: 60.0, height: 1080 };
        let mut glows = Vec::new();
        let mut lenses = Vec::new();
        append(&mut glows, &mut lenses, (Vec3::X * 30.0, 1.0, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO), view, 1.0);
        assert_eq!(glows.len(), 12, "an unsplit lantern keeps its halo and core");
        assert!(lenses.is_empty(), "no lens panel is guessed for a lantern");
        glows.clear();
        append(&mut glows, &mut lenses, (Vec3::X * 30.0, 1.0, Vec3::ZERO, Vec3::ZERO, Vec3::Z), view, 1.0);
        assert!(glows.is_empty() && lenses.is_empty(), "emissive glass replaces the sprite");
    }
}
