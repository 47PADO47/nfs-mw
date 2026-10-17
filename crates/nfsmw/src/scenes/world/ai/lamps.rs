//! The traffic lights drawn in the world. An approach whose light is a real traffic-light prop of the map (matched in
//! `signal_props.rs`) shows its state as a lamp on that prop's lamp head, and no post or bar of its own. An approach
//! with no matched prop keeps the synthetic marker: a post at the kerb with a lamp on top, and a bar across the road
//! at the stop line. The debug switch `signals synthetic on` draws the synthetic marker for every approach. A matched
//! prop that is knocked over shows nothing, its marker included, until it stands again.
//! They are a rewrite extension together with the lights themselves.

use blackbox_gfx::RenderBackend;
use blackbox_render::{EffectVertex, Instance, MeshHandle};
use blackbox_roads::{Light, right_of};
use glam::{Mat4, Vec3};

use super::TrafficWorld;
use crate::scenes::world::drive::upload_box;
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::space;
use crate::scenes::world::vehicle_effects::geometry;

/// Lamps further than this from the camera (metres) are not drawn.
const DRAW_DISTANCE: f32 = 300.0;
/// How far beyond the kerb the post stands, metres.
const POST_SETBACK: f32 = 1.0;
/// Height of the post and the edge length of the lamp on top of it, metres.
const POST_HEIGHT: f32 = 5.0;
const POST_WIDTH: f32 = 0.25;
const LAMP_SIZE: f32 = 0.9;
/// The lamp on a matched prop's head: edge length and glow radius, metres.
const HEAD_SIZE: f32 = 0.35;
const HEAD_GLOW: f32 = 0.6;
/// The bar across the road at the stop line: how deep (along the road), how high above the surface, metres.
const BAR_DEPTH: f32 = 0.4;
const BAR_HEIGHT: f32 = 0.05;
/// Vertex colours, B G R A: the world is pre-lit with 0x80 as full brightness.
const GREEN: [u8; 4] = [0x20, 0x80, 0x20, 0xFF];
const AMBER: [u8; 4] = [0x10, 0x70, 0x80, 0xFF];
const RED: [u8; 4] = [0x18, 0x18, 0x80, 0xFF];
const POST: [u8; 4] = [0x40, 0x40, 0x40, 0xFF];
const BAR: [u8; 4] = [0x80, 0x80, 0x80, 0xFF];
/// Glow brightness, and the lamp colours as RGBA.
const GLOW_BRIGHTNESS: f32 = 0.9;
const GLOW_GREEN: [f32; 4] = [0.2, 1.0, 0.3, 1.0];
const GLOW_AMBER: [f32; 4] = [1.0, 0.65, 0.1, 1.0];
const GLOW_RED: [f32; 4] = [1.0, 0.1, 0.05, 1.0];
/// Glow radius (metres) over a synthetic lamp.
const LAMP_GLOW: f32 = 1.1;

/// The meshes the lamps are drawn with.
pub struct LampMeshes {
    green: MeshHandle,
    amber: MeshHandle,
    red: MeshHandle,
    post: MeshHandle,
    bar: MeshHandle,
}

impl LampMeshes {
    pub fn upload(renderer: &mut dyn RenderBackend) -> Self {
        Self {
            green: upload_box(renderer, "signal green", GREEN),
            amber: upload_box(renderer, "signal amber", AMBER),
            red: upload_box(renderer, "signal red", RED),
            post: upload_box(renderer, "signal post", POST),
            bar: upload_box(renderer, "stop line", BAR),
        }
    }

    /// Free the meshes.
    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for mesh in [self.green, self.amber, self.red, self.post, self.bar] {
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

/// What an approach shows: its matched light (when it stands, not knocked over) and whether the synthetic post and
/// bar stand.
struct Shown {
    head: Option<super::signal_props::Lamp>,
    post: bool,
}

impl TrafficWorld {
    /// What the approach `index` shows. A matched prop that is knocked over shows nothing at all.
    fn shown(&self, index: usize, props: &PropWorld) -> Shown {
        let lamp = self.signal_lamps.lamp(index);
        let knocked = lamp.and_then(|lamp| lamp.prop).is_some_and(|id| props.hidden(id));
        if knocked {
            return Shown { head: None, post: false };
        }
        Shown { head: lamp, post: lamp.is_none() || self.signal_lamps.synthetic_all }
    }

    /// Appends a glow over each signal lamp within sight, in the colour it shows: on the matched prop's head, or on
    /// the synthetic post.
    pub fn lamp_glows(&self, camera: Vec3, forward: Vec3, props: &PropWorld, out: &mut Vec<EffectVertex>) {
        let up = Vec3::Z;
        for (index, approach) in self.signals.approaches().iter().enumerate() {
            let shown = self.shown(index, props);
            let colour = glow_colour(self.signals.state(index, self.signal_time));
            if let Some(lamp) = shown.head.filter(|lamp| near(lamp.position, camera)) {
                geometry::glow(
                    out,
                    lamp.position,
                    HEAD_GLOW,
                    geometry::color(colour, GLOW_BRIGHTNESS),
                    camera,
                    forward,
                );
            }
            if !shown.post {
                continue;
            }
            let stop = space::to_render(approach.stop_position.to_array());
            if !near(stop, camera) {
                continue;
            }
            let right = space::to_render(right_of(approach.heading).to_array());
            let post = stop + right * (approach.kerb + POST_SETBACK);
            geometry::glow(
                out,
                post + up * POST_HEIGHT,
                LAMP_GLOW,
                geometry::color(colour, GLOW_BRIGHTNESS),
                camera,
                forward,
            );
        }
    }

    /// Appends the instances of the signals within sight of `camera` (render space), when there are meshes for them:
    /// the lamp on the head of every matched prop, and for the approaches with the synthetic marker the post, the lamp
    /// on top of it and the stop bar.
    pub fn lamp_instances(
        &self,
        meshes: Option<&LampMeshes>,
        camera: Vec3,
        props: &PropWorld,
        out: &mut Vec<Instance>,
    ) {
        let Some(meshes) = meshes else { return };
        let up = Vec3::Z;
        for (index, approach) in self.signals.approaches().iter().enumerate() {
            let shown = self.shown(index, props);
            let light = self.signals.state(index, self.signal_time);
            if let Some(lamp) = shown.head.filter(|lamp| near(lamp.position, camera)) {
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
                });
            }
            if !shown.post {
                continue;
            }
            let stop = space::to_render(approach.stop_position.to_array());
            if !near(stop, camera) {
                continue;
            }
            let heading = space::to_render(approach.heading.to_array());
            let right = space::to_render(right_of(approach.heading).to_array());
            let post = stop + right * (approach.kerb + POST_SETBACK);
            out.push(Instance {
                mesh: meshes.post,
                transform: oriented_box(post, up, heading, Vec3::new(POST_HEIGHT, POST_WIDTH, POST_WIDTH)),
                key: blackbox_gfx::InstanceKey::TRANSIENT,
                shadow: Default::default(),
            });
            out.push(Instance {
                mesh: meshes.lamp(light),
                transform: oriented_box(post + up * POST_HEIGHT, up, heading, Vec3::splat(LAMP_SIZE)),
                key: blackbox_gfx::InstanceKey::TRANSIENT,
                shadow: Default::default(),
            });
            // The bar runs from the centre line to the kerb, just above the surface.
            out.push(Instance {
                mesh: meshes.bar,
                transform: oriented_box(
                    stop + up * (BAR_HEIGHT / 2.0),
                    right,
                    heading,
                    Vec3::new(approach.kerb, BAR_DEPTH, BAR_HEIGHT),
                ),
                key: blackbox_gfx::InstanceKey::TRANSIENT,
                shadow: Default::default(),
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
