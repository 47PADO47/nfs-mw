//! The traffic lights drawn in the world: a post at the kerb of every signalled approach with a lamp on it that
//! shows the light, and a bar across the road at the stop line. The signal scenery of the map (`XO_TrafficLight*`
//! and the like) is drawn as ordinary props and cannot be recoloured, so these markers show what the cars obey.
//! They are a rewrite extension together with the lights themselves.

use blackbox_render::{Instance, MeshHandle, Renderer};
use blackbox_roads::{Light, right_of};
use glam::{Mat4, Vec3};

use super::TrafficWorld;
use crate::scenes::world::drive::upload_box;
use crate::scenes::world::space;

/// Lamps further than this from the camera (metres) are not drawn.
const DRAW_DISTANCE: f32 = 300.0;
/// How far beyond the kerb the post stands, metres.
const POST_SETBACK: f32 = 1.0;
/// Height of the post and the edge length of the lamp on top of it, metres.
const POST_HEIGHT: f32 = 5.0;
const POST_WIDTH: f32 = 0.25;
const LAMP_SIZE: f32 = 0.9;
/// The bar across the road at the stop line: how deep (along the road), how high above the surface, metres.
const BAR_DEPTH: f32 = 0.4;
const BAR_HEIGHT: f32 = 0.05;
/// Vertex colours, B G R A: the world is pre-lit with 0x80 as full brightness.
const GREEN: [u8; 4] = [0x20, 0x80, 0x20, 0xFF];
const AMBER: [u8; 4] = [0x10, 0x70, 0x80, 0xFF];
const RED: [u8; 4] = [0x18, 0x18, 0x80, 0xFF];
const POST: [u8; 4] = [0x40, 0x40, 0x40, 0xFF];
const BAR: [u8; 4] = [0x80, 0x80, 0x80, 0xFF];

/// The meshes the lamps are drawn with.
pub struct LampMeshes {
    green: MeshHandle,
    amber: MeshHandle,
    red: MeshHandle,
    post: MeshHandle,
    bar: MeshHandle,
}

impl LampMeshes {
    pub fn upload(renderer: &mut Renderer) -> Self {
        Self {
            green: upload_box(renderer, "signal green", GREEN),
            amber: upload_box(renderer, "signal amber", AMBER),
            red: upload_box(renderer, "signal red", RED),
            post: upload_box(renderer, "signal post", POST),
            bar: upload_box(renderer, "stop line", BAR),
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

impl TrafficWorld {
    /// Appends the instances of the signals within sight of `camera` (render space), when there are meshes for them: the post, the lamp in the
    /// colour of the light and the stop bar of every signalled approach.
    pub fn lamp_instances(&self, meshes: Option<&LampMeshes>, camera: Vec3, out: &mut Vec<Instance>) {
        let Some(meshes) = meshes else { return };
        let up = Vec3::Z;
        for (index, approach) in self.signals.approaches().iter().enumerate() {
            let stop = space::to_render(approach.stop_position.to_array());
            if (stop - camera).truncate().length() > DRAW_DISTANCE {
                continue;
            }
            let heading = space::to_render(approach.heading.to_array());
            let right = space::to_render(right_of(approach.heading).to_array());
            let post = stop + right * (approach.kerb + POST_SETBACK);
            let light = self.signals.state(index, self.signal_time);
            out.push(Instance {
                mesh: meshes.post,
                transform: oriented_box(post, up, heading, Vec3::new(POST_HEIGHT, POST_WIDTH, POST_WIDTH)),
            });
            out.push(Instance {
                mesh: meshes.lamp(light),
                transform: oriented_box(post + up * POST_HEIGHT, up, heading, Vec3::splat(LAMP_SIZE)),
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
}
