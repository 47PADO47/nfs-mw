//! Roadblocks in the world: a block of cop cars across the road 250 m ahead of the player, laid out by
//! `blackbox_pursuit::roadblock`. The cars stop where they are placed, with the brakes held (`AIGoalStaticRoadBlock`).
//! The cars and the spike strips are placed; the sawhorse barriers are not, as they need props. A roadblock rolls its
//! layout list by the heat's spike chance. The road is taken as two lanes (a 3.5 m half width, unconfirmed) because the
//! rewrite does not read the lane profile here yet. Spec: `docs/specs/ai-pursuit-roadblocks.md` sections 2 to 4.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::roadblock::{self, Element, Layout, NORMAL, Road, SPIKES};
use blackbox_pursuit::spikes::Strip;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::{Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficWorld;
use super::car::{AiCar, Role, Start};
use super::traffic::Cruise;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// How far ahead of the player the roadblock stands, metres (`kRoadBlockAheadDistance`).
const AHEAD: f32 = 250.0;
/// Half the width of the road (two lanes), metres (unconfirmed).
const LANE_HALF: f32 = 3.5;

impl TrafficWorld {
    /// Lays a roadblock 250 m ahead of the player. Returns the number of cop cars placed.
    pub(in crate::scenes::world) fn place_roadblock(
        &mut self,
        view: super::PlayerView,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Result<usize, String> {
        let at_player = Vec3::from_array(space::to_physics(view.position));
        let forward = Vec3::from_array(space::to_physics(view.forward)).with_y(0.0).normalize_or_zero();
        if self.cop_models.is_empty() {
            return Err("no cop model is loaded (start a pursuit first)".into());
        }
        let mut nav = RoadNav::new(
            NavKind::Direction,
            LaneType::Racing,
            SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() },
        );
        if !nav.init_at_point(&self.network, &self.index, at_player + forward * AHEAD, forward, false) {
            return Err("there is no road 250 m ahead".into());
        }
        let side = Vec3::new(forward.z, 0.0, -forward.x);
        let centre = nav.position;
        let road =
            Road { left: centre + side * LANE_HALF, right: centre - side * LANE_HALF, target: at_player, forward };
        let cr_width = 2.0 * LANE_HALF + 1.0;
        let cars = roadblock::cars_asked(cr_width);
        let heat = self.police.level().unwrap_or(1);
        let spikes = roadblock::takes_spikes(heat, self.rng.next_f32());
        let list: &[Layout] = if spikes { &SPIKES } else { &NORMAL };
        let layout = roadblock::pick(list, cr_width, cars).ok_or("no roadblock layout fits this road")?;
        let mut placed = 0;
        let mut strips = 0;
        for slot in roadblock::place(layout, &road) {
            let at = Vec3::new(slot.position.x, centre.y, slot.position.z);
            match slot.kind {
                Element::Car => {
                    if self.place_roadblock_car(at, slot.heading, collision, surfaces) {
                        placed += 1;
                    }
                }
                Element::Spike => {
                    self.spikes.push(Strip { position: Vec2::new(at.x, at.z), elevation: at.y, heading: slot.heading });
                    strips += 1;
                }
                // Sawhorse barriers need their props, which are not spawned yet.
                Element::Barrier => {}
            }
        }
        log::info!("roadblock: {} layout, {strips} spike strips", if spikes { "spike" } else { "plain" });
        Ok(placed)
    }

    /// Places one roadblock car at a physics-space point, facing `heading` (radians in the physics x-z plane).
    fn place_roadblock_car(
        &mut self,
        at: Vec3,
        heading: f32,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let direction = Vec3::new(heading.cos(), 0.0, heading.sin());
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, at, direction, false) {
            log::warn!("roadblock: no road at ({:.0}, {:.0})", at.x, at.z);
            return false;
        }
        let render_forward = space::to_render(direction.to_array());
        // Snap to the road: the nav gives the nearest road point, as the traffic spawn does; a point beside the road has no ground.
        let on_road = nav.position;
        let render_at = space::to_render(on_road.to_array());
        let spawn = Spawn { position: render_at, heading: render_forward.y.atan2(render_forward.x) };
        let start = Start {
            stagger: 0,
            speed: 0.0,
            cruise: Cruise { street: 0.0, highway: 0.0, lateral_g: 1.6, limit_acceleration: false },
            patrol: false,
        };
        let model = &self.cop_models[0].model;
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        let Some(mut car) = AiCar::place(model, Role::Traffic, nav, spawn, start, &WorldGround { collision, surfaces })
        else {
            log::warn!("roadblock: the car does not fit at ({:.0}, {:.0})", at.x, at.z);
            return false;
        };
        car.set_roadblock();
        log::info!("roadblock: {} at ({:.0}, {:.0})", car.name, render_at.x, render_at.y);
        self.cars.push(car);
        true
    }
}
