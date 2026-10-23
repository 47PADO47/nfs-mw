//! The traffic lights drawn in the world. An approach shows its state as a lamp on the lamp head of the real
//! traffic-light prop the matcher gave it (`signal_props.rs`). An approach with no matched prop shows no light: the
//! rewrite draws no lights of its own on the map. A prop that is knocked over shows nothing until it stands again.
//! They are a rewrite extension together with the lights themselves.

use blackbox_gfx::RenderBackend;
use blackbox_render::{EffectVertex, Instance, MeshHandle};
use blackbox_roads::Light;
use glam::{Mat4, Vec3};

use super::TrafficWorld;
use crate::scenes::world::drive::upload_box;
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::vehicle_effects::geometry;

/// Lamps further than this from the camera (metres) are not drawn.
const DRAW_DISTANCE: f32 = 300.0;
/// The lamp on a matched prop's head: edge length and glow radius, metres.
const HEAD_SIZE: f32 = 0.35;
const HEAD_GLOW: f32 = 0.6;
/// Vertex colours, B G R A: the world is pre-lit with 0x80 as full brightness.
const GREEN: [u8; 4] = [0x20, 0x80, 0x20, 0xFF];
const AMBER: [u8; 4] = [0x10, 0x70, 0x80, 0xFF];
const RED: [u8; 4] = [0x18, 0x18, 0x80, 0xFF];
/// Glow brightness, and the lamp colours as RGBA.
const GLOW_BRIGHTNESS: f32 = 0.9;
const GLOW_GREEN: [f32; 4] = [0.2, 1.0, 0.3, 1.0];
const GLOW_AMBER: [f32; 4] = [1.0, 0.65, 0.1, 1.0];
const GLOW_RED: [f32; 4] = [1.0, 0.1, 0.05, 1.0];

/// The meshes the lamps are drawn with.
pub struct LampMeshes {
    green: MeshHandle,
    amber: MeshHandle,
    red: MeshHandle,
}

impl LampMeshes {
    pub fn upload(renderer: &mut dyn RenderBackend) -> Self {
        Self {
            green: upload_box(renderer, "signal green", GREEN),
            amber: upload_box(renderer, "signal amber", AMBER),
            red: upload_box(renderer, "signal red", RED),
        }
    }

    /// Free the meshes.
    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for mesh in [self.green, self.amber, self.red] {
            renderer.destroy_mesh(mesh);
        }
    }

    fn lamp(&self, light: Light) -> MeshHandle {
        match light {
            Light::Green => self.green,
            Light::Amber => self.amber,
            Light::Red => self.red,
        }
    }
}

/// A box from `origin`, `size.x` long along `forward`, `size.y` wide along `side` and `size.z` along the third
/// axis, both centred on the line from `origin` (the shape of the unit mesh).
fn oriented_box(origin: Vec3, forward: Vec3, side: Vec3, size: Vec3) -> Mat4 {
    let third = forward.cross(side).normalize_or(Vec3::Z);
    Mat4::from_cols(
        (forward * size.x).extend(0.0),
        (side * size.y).extend(0.0),
        (third * size.z).extend(0.0),
        origin.extend(1.0),
    )
}

/// The glow colour of a light.
fn glow_colour(light: Light) -> [f32; 4] {
    match light {
        Light::Green => GLOW_GREEN,
        Light::Amber => GLOW_AMBER,
        Light::Red => GLOW_RED,
    }
}

/// Whether a point (render space) is within sight of the camera.
fn near(point: Vec3, camera: Vec3) -> bool {
    (point - camera).truncate().length() <= DRAW_DISTANCE
}

impl TrafficWorld {
    /// The lamp an approach shows, if any. A matched prop that is knocked over shows nothing at all.
    fn shown(&self, index: usize, props: &PropWorld) -> Option<super::signal_props::Lamp> {
        let lamp = self.signal_lamps.lamp(index)?;
        let knocked = lamp.prop.is_some_and(|id| props.hidden(id));
        (!knocked).then_some(lamp)
    }

    /// Appends a glow over each signal lamp within sight, in the colour it shows, on the matched prop's head.
    pub fn lamp_glows(&self, camera: Vec3, forward: Vec3, props: &PropWorld, out: &mut Vec<EffectVertex>) {
        for index in 0..self.signals.approaches().len() {
            let Some(lamp) = self.shown(index, props).filter(|lamp| near(lamp.position, camera)) else { continue };
            let colour = glow_colour(self.signals.state(index, self.signal_time));
            geometry::glow(out, lamp.position, HEAD_GLOW, geometry::color(colour, GLOW_BRIGHTNESS), camera, forward);
        }
    }

    /// Appends the lamp instances within sight of `camera` (render space), when there are meshes for them: the lamp on
    /// the head of every matched prop.
    pub fn lamp_instances(
        &self,
        meshes: Option<&LampMeshes>,
        camera: Vec3,
        props: &PropWorld,
        out: &mut Vec<Instance>,
    ) {
        let Some(meshes) = meshes else { return };
        let up = Vec3::Z;
        for index in 0..self.signals.approaches().len() {
            let Some(lamp) = self.shown(index, props).filter(|lamp| near(lamp.position, camera)) else { continue };
            let light = self.signals.state(index, self.signal_time);
            out.push(Instance {
                mesh: meshes.lamp(light),
                transform: oriented_box(
                    lamp.position - up * (HEAD_SIZE / 2.0),
                    up,
                    lamp.forward,
                    Vec3::splat(HEAD_SIZE),
                ),
                key: blackbox_gfx::InstanceKey::TRANSIENT,
                shadow: Default::default(),
                flip_winding: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_runs_from_its_origin_along_forward_and_is_centred_across() {
        let t = oriented_box(Vec3::new(1.0, 2.0, 3.0), Vec3::Z, Vec3::X, Vec3::new(5.0, 0.5, 0.25));
        assert!(t.transform_point3(Vec3::ZERO).distance(Vec3::new(1.0, 2.0, 3.0)) < 1e-5);
        assert!(t.transform_point3(Vec3::X).distance(Vec3::new(1.0, 2.0, 8.0)) < 1e-5, "5 m along forward");
        // Half the unit width along `side` (x) is 0.25 m.
        let edge = t.transform_point3(Vec3::new(0.0, 0.5, 0.0));
        assert!((edge.x - 1.25).abs() < 1e-5, "{edge:?}");
    }

    #[test]
    fn a_lamp_is_drawn_only_within_sight() {
        let camera = Vec3::ZERO;
        assert!(near(Vec3::new(100.0, 0.0, 50.0), camera), "height does not count against sight");
        assert!(!near(Vec3::new(DRAW_DISTANCE + 1.0, 0.0, 0.0), camera));
    }

    #[test]
    fn each_light_has_its_own_glow_colour() {
        assert_eq!(glow_colour(Light::Green), GLOW_GREEN);
        assert_eq!(glow_colour(Light::Amber), GLOW_AMBER);
        assert_eq!(glow_colour(Light::Red), GLOW_RED);
    }
}
