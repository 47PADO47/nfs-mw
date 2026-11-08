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

/// Where the lamp of `light` shows on a head: the lamp point, moved up for red and down for green by the pitch of the lenses of a three-lens
/// head (`signal_props::HEAD_C`), so the lit lens is the one the colour belongs to. A head with no lens pitch shows it at the point.
fn lit_position(lamp: &super::signal_props::Lamp, light: Light) -> Vec3 {
    let lens = match light {
        Light::Red => 1.0,
        Light::Amber => 0.0,
        Light::Green => -1.0,
    };
    lamp.position + Vec3::Z * (lamp.lens_pitch * lens)
}

/// Whether a point (render space) is within sight of the camera.
fn near(point: Vec3, camera: Vec3) -> bool {
    (point - camera).truncate().length() <= DRAW_DISTANCE
}

impl TrafficWorld {
    /// Appends nearby traffic beams, fading both the range and count boundary.
    pub(in crate::scenes::world) fn local_headlights(
        &self,
        camera: Vec3,
        strength: f32,
        limit: u32,
        out: &mut Vec<blackbox_gfx::LocalLight>,
    ) {
        let mut nearby: Vec<_> = self
            .cars
            .iter()
            .map(|car| {
                let distance = car.pose(self.alpha).transform().w_axis.truncate().distance_squared(camera);
                (distance, car.key(), car)
            })
            .filter(|(distance, _, car)| *distance < 150.0 * 150.0 && car.has_local_headlights())
            .collect();
        nearby.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let limit = limit.min(8) as usize;
        let boundary = nearby.get(limit).map_or(150.0, |(distance, _, _)| distance.sqrt());
        for (distance, _, car) in nearby.into_iter().take(limit) {
            let fade = ((boundary - distance.sqrt()) / 24.0).clamp(0.0, 1.0);
            let fade = fade * fade * (3.0 - 2.0 * fade);
            car.local_headlights(self.alpha, strength * fade, out);
        }
    }

    /// The colour the lamp of an approach is shown in: its own, or the one `signals force` asks for.
    fn signal_light(&self, index: usize) -> Light {
        self.signal_force.unwrap_or_else(|| self.signals.state(index, self.signal_time))
    }

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
            let light = self.signal_light(index);
            let colour = glow_colour(light);
            geometry::glow(
                out,
                lit_position(&lamp, light),
                HEAD_GLOW,
                geometry::color(colour, GLOW_BRIGHTNESS),
                camera,
                forward,
            );
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
            let light = self.signal_light(index);
            out.push(Instance {
                mesh: meshes.lamp(light),
                transform: oriented_box(
                    lit_position(&lamp, light) - up * (HEAD_SIZE / 2.0),
                    up,
                    lamp.forward,
                    Vec3::splat(HEAD_SIZE),
                ),
                key: blackbox_gfx::InstanceKey::TRANSIENT,
                shadow: Default::default(),
                emission: 0.0,
                owner_key: blackbox_gfx::InstanceKey::TRANSIENT,
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
    fn the_lit_lens_is_red_on_top_amber_in_the_middle_and_green_below() {
        let lamp = super::super::signal_props::Lamp {
            position: Vec3::new(1.0, 2.0, 3.0),
            forward: Vec3::X,
            prop: None,
            lens_pitch: 0.4,
        };
        assert_eq!(lit_position(&lamp, Light::Amber), lamp.position);
        assert!((lit_position(&lamp, Light::Red).z - 3.4).abs() < 1e-6);
        assert!((lit_position(&lamp, Light::Green).z - 2.6).abs() < 1e-6);
        let flat = super::super::signal_props::Lamp { lens_pitch: 0.0, ..lamp };
        assert_eq!(lit_position(&flat, Light::Red), flat.position, "a head with no lenses lights its point");
    }

    #[test]
    fn each_light_has_its_own_glow_colour() {
        assert_eq!(glow_colour(Light::Green), GLOW_GREEN);
        assert_eq!(glow_colour(Light::Amber), GLOW_AMBER);
        assert_eq!(glow_colour(Light::Red), GLOW_RED);
    }
}
