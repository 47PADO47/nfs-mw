//! Roadblocks in the world: a block of cop cars across the road 250 m ahead of the player, laid out by
//! `blackbox_pursuit::roadblock`. The cars stop where they are placed, with the brakes held (`AIGoalStaticRoadBlock`).
//! The cars and the spike strips are placed; the sawhorse barriers are not, as they need props. A roadblock rolls its
//! layout list by the heat's spike chance. The road's navigable corridor supplies the layout width and tangent.
//! Spec: `docs/specs/ai-pursuit-roadblocks.md` sections 2 to 4.

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
        if !nav.init_at_point(&self.network, &self.index, at_player, forward, false) {
            return Err("there is no road 250 m ahead".into());
        }
        nav.half_width = 0.0;
        nav.advance(&self.network, AHEAD, forward, &mut self.rng);
        if nav.dead_end || nav.position.distance(at_player) < 100.0 {
            return Err("the road does not provide a safe roadblock site ahead".into());
        }
        let forward = nav.forward.with_y(0.0).normalize_or_zero();
        let cross_section = nav.road_cross_section(&self.network);
        let centre = nav.position;
        let lift = |point: Vec2| Vec3::new(point.x, centre.y, point.y);
        let road =
            Road { left: lift(cross_section.left), right: lift(cross_section.right), target: at_player, forward };
        let cr_width = cross_section.left.distance(cross_section.right);
        let cars = roadblock::cars_asked(cr_width);
        let heat = self.police.level().unwrap_or(1);
        let spikes = roadblock::takes_spikes(heat, self.rng.next_f32());
        let list: &[Layout] = if spikes { &SPIKES } else { &NORMAL };
        let layout = roadblock::pick(list, cr_width, cars).ok_or("no roadblock layout fits this road")?;
        let mut placed = 0;
        let mut strips = Vec::new();
        for slot in roadblock::place(layout, &road) {
            let at = Vec3::new(slot.position.x, centre.y, slot.position.z);
            match slot.kind {
                Element::Car => {
                    if self.place_roadblock_car(at, slot.heading, collision, surfaces) {
                        placed += 1;
                    }
                }
                Element::Spike => {
                    strips.push(Strip { position: Vec2::new(at.x, at.z), elevation: at.y, heading: slot.heading });
                }
                // Sawhorse barriers need their props, which are not spawned yet.
                Element::Barrier => {}
            }
        }
        if placed == 0 {
            return Err("no roadblock car could stand at the selected site".into());
        }
        log::info!("roadblock: {} layout, {} spike strips", if spikes { "spike" } else { "plain" }, strips.len());
        self.spikes.extend(strips);
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
        // Ground placement resolves elevation; do not discard the layout's lateral and longitudinal offsets.
        let render_at = space::to_render(at.to_array());
        let spawn = Spawn { position: render_at, heading: render_forward.y.atan2(render_forward.x) };
        let start = Start {
            stagger: 0,
            speed: 0.0,
            cruise: Cruise { street: 0.0, highway: 0.0, lateral_g: 1.6, limit_acceleration: false },
            patrol: false,
        };
        let model = self
            .pursuit
            .as_ref()
            .into_iter()
            .flat_map(|p| p.wave())
            .find_map(|(name, _, _)| {
                self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)).map(|m| &m.model)
            })
            .unwrap_or(&self.cop_models[0].model);
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
