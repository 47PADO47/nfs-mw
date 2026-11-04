//! Building a roadblock (`CreateRoadBlock`, spec sections 2.2 to 2.5): the site, the layout for its width, the cars, the
//! props. A roadblock is built whole or not at all: every car must stand and every element must lie on the road and clear
//! of other vehicles, or the attempt fails and the request tries again on the next manager run.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::roadblock::{self, Element, Layout, NORMAL, Placed, SPIKES};
use blackbox_pursuit::spikes::{self, Strip};
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use blackbox_vehicle::Ground;
use glam::{Mat4, Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;
use std::rc::Rc;

use super::site::Site;
use super::spacing;
use super::state::Roadblock;
use crate::scenes::world::ai::car::{AiCar, Role, Start};
use crate::scenes::world::ai::support;
use crate::scenes::world::ai::traffic::Cruise;
use crate::scenes::world::ai::{PlayerView, TrafficWorld};
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// Why a roadblock is being built; it picks the cars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cause {
    /// The request lottery (`RequestRoadBlock`): cars from the cop pool, here the wave's models.
    Lottery,
    /// The heavy support option `HEAVY_ROADBLOCK`: SUVs, the big one with this percent chance.
    Heavy { big_suv: f32 },
    /// The `roadblock` console command.
    Console,
}

/// A car is this near another vehicle's slot or an element's slot (metres) when the site counts as occupied.
const CLEARANCE: f32 = 4.5;
/// The player's car is this near any element (metres) when the site counts as occupied.
const PLAYER_CLEARANCE: f32 = 14.0;
/// How far an element may stand beyond the road's edge, metres. The layouts reach about half a metre past their nominal
/// width, the width rule adds a metre, and the layout stretches by up to 14 percent.
const EDGE_SLACK: f32 = 1.5;
/// How far along the road an element may stand from the site, metres (the layouts reach about 6 m).
const ALONG_LIMIT: f32 = 14.0;

/// A model for a roadblock car: loaded cop models only. The index is into `cop_models`.
fn pick_model(world: &mut TrafficWorld, cause: Cause) -> Option<usize> {
    let loaded = |name: &str| world.cop_models.iter().position(|m| m.model.name.eq_ignore_ascii_case(name));
    if let Cause::Heavy { big_suv } = cause {
        let roll = world.rng.roadblock.next_f32();
        let name = support::car_model(blackbox_pursuit::support::cars::suv(big_suv, roll));
        if let Some(index) = loaded(name) {
            return Some(index);
        }
    }
    let wave: Vec<usize> =
        world.pursuit.iter().flat_map(|p| p.wave()).filter_map(|(name, _, _)| loaded(name)).collect();
    match wave.is_empty() {
        true => (!world.cop_models.is_empty()).then_some(0),
        false => Some(wave[world.rng.roadblock.index(wave.len())]),
    }
}

/// How far above and below an element's point the nearest surface is looked for first, metres.
const NEAR: f32 = 2.0;
/// How far above the point the fallback search reaches, metres (the original's segment runs 1000 m up).
const FALLBACK_REACH: f32 = 1002.0;
/// The most placeable props (sawhorses and strips) standing at once; a roadblock's props past it are not created.
const MAX_PLACEABLES: usize = 13;

/// The world surface nearest a physics-space point that carries the road's height (spec section 2.5): the surface
/// within `NEAR` metres of it, found going down from `NEAR` above, else the first one going up from `NEAR` below. Its
/// height and its upward normal (a normal that points down is flipped). `None` when there is no surface there.
fn surface_near(ground: &impl Ground, at: Vec3) -> Option<(f32, Vec3)> {
    let up = |normal: Vec3| match normal.y < 0.0 {
        true => -normal,
        false => normal,
    };
    let top = at + Vec3::Y * NEAR;
    if let Some(hit) = ground.hit(top, Vec3::NEG_Y, 2.0 * NEAR) {
        return Some((top.y - hit.distance, up(hit.normal)));
    }
    let bottom = at - Vec3::Y * NEAR;
    let hit = ground.hit(bottom, Vec3::Y, FALLBACK_REACH)?;
    Some((bottom.y + hit.distance, up(hit.normal)))
}

/// How far above the road the walls around a roadblock car are looked for, metres: about a bumper's height.
const WALL_PROBE_HEIGHT: f32 = 0.8;

/// A wall (a face or barrier standing more than 60 degrees from level) on the level segment from `from` to `to` at
/// `height` (physics space): the fraction of the way and its horizontal normal on the side of `from`.
fn wall_between(collision: &CollisionWorld, from: Vec2, to: Vec2, height: f32) -> Option<(f32, Vec2)> {
    let start = Vec3::new(from.x, height, from.y);
    let end = Vec3::new(to.x, height, to.y);
    let options = blackbox_collision::RayOptions::default();
    let hit =
        collision.ray_cast_filtered(start.to_array(), end.to_array(), &options, |hit| hit.normal[1].abs() < 0.5)?;
    let normal = Vec2::new(hit.normal[0], hit.normal[2]).normalize_or_zero();
    let along = Vec2::new(hit.point[0], hit.point[2]).distance(from) / from.distance(to).max(1e-3);
    Some((along, normal))
}

/// [ours] The plane of the road under a prop, from the surface at its four ends: `reach` is half its extent along its
/// facing and across it. One face's normal under the centre can belong to a kerb or a seam, and a 6.5 m strip tilted
/// by it sank up to half a metre into the road at one end. The height is the plane's at the centre, never below the
/// surface found there. `None` when an end has no surface near it.
fn fitted_surface(ground: &impl Ground, centre: Vec3, heading: f32, reach: [f32; 2]) -> Option<(f32, Vec3)> {
    let facing = Vec3::new(heading.cos(), 0.0, heading.sin());
    let across = Vec3::new(-facing.z, 0.0, facing.x);
    let at = |offset: Vec3| surface_near(ground, centre + offset).map(|(y, _)| (centre + offset).with_y(y));
    let (ahead, behind) = (at(facing * reach[0])?, at(-facing * reach[0])?);
    let (left, right) = (at(across * reach[1])?, at(-across * reach[1])?);
    let normal = (ahead - behind).cross(left - right).normalize_or_zero();
    let normal = match normal.y < 0.0 {
        true => -normal,
        false => normal,
    };
    if normal.y < 0.5 {
        return None;
    }
    let plane = (ahead.y + behind.y + left.y + right.y) / 4.0;
    Some((plane.max(centre.y), normal))
}

impl TrafficWorld {
    /// Builds a roadblock ahead of the player. `props` is the prop world for the sawhorses; without it (the headless
    /// tests) the sawhorses are left out. Returns the number of cop cars placed.
    pub(in crate::scenes::world) fn build_roadblock(
        &mut self,
        cause: Cause,
        view: PlayerView,
        collision: &CollisionWorld,
        mut props: Option<&mut PropWorld>,
        surfaces: &SurfaceTable,
    ) -> Result<usize, String> {
        if self.cop_models.is_empty() {
            return Err("no cop model is loaded (start a pursuit first)".into());
        }
        let target = Vec3::from_array(space::to_physics(view.position));
        let heading = Vec3::from_array(space::to_physics(view.forward)).with_y(0.0).normalize_or_zero();
        let site = self.roadblock_site(target, heading)?;
        let asked = roadblock::cars_asked(site.cr_width);
        let level = self.police.level().unwrap_or(1);
        let cooling = self.police.cooldown_status() == blackbox_pursuit::CooldownStatus::CoolDown;
        let spikes = !cooling && self.rng.roadblock.next_f32() * 100.0 < self.police.roadblock_odds(level).spike_chance;
        let list: &[Layout] = if spikes { &SPIKES } else { &NORMAL };
        let layout = roadblock::pick(list, site.cr_width, asked).ok_or("no roadblock layout fits this road")?;
        let mut placed = roadblock::place(layout, &site.road);
        let car_slots = placed.iter().filter(|slot| slot.kind == Element::Car).count();
        // The lottery's cars come from cops out of view first (the cop manager's steal rule), then are created.
        let mut thieves = match cause {
            Cause::Lottery => self.steal_candidates(view.position, car_slots),
            Cause::Heavy { .. } | Cause::Console => Vec::new(),
        }
        .into_iter();
        let mut taken = Vec::new();
        let mut models = Vec::with_capacity(car_slots);
        for _ in 0..car_slots {
            let stolen = thieves.next().and_then(|thief| {
                let index = self.cop_models.iter().position(|m| m.model.name.eq_ignore_ascii_case(&thief.model))?;
                Some((index, thief.key))
            });
            let model = match stolen {
                Some((index, key)) => {
                    taken.push(key);
                    index
                }
                None => pick_model(self, cause).ok_or("no cop model for the roadblock")?,
            };
            models.push(model);
        }
        self.space_cars(&mut placed, &models, collision);
        self.check_clear(&placed, &site, target)?;
        let ground = WorldGround { collision, surfaces };
        let mut cars = Vec::new();
        for (slot, &model) in placed.iter().filter(|slot| slot.kind == Element::Car).zip(&models) {
            let car = self.roadblock_car(model, slot, &ground).ok_or_else(|| {
                format!("a roadblock car cannot stand at ({:.0}, {:.0})", slot.position.x, slot.position.z)
            })?;
            cars.push(car);
        }
        let id = self.roadblocks.next_id + 1;
        self.roadblocks.next_id = id;
        let mut block = Roadblock {
            id,
            standing: roadblock::standing::Standing::new(roadblock::centre(layout, &site.road), site.forward, 0),
            strips: Vec::new(),
            strip_frames: Vec::new(),
            sawhorses: Vec::new(),
            owner: Rc::new(()),
            recruited: 0,
        };
        let count = cars.len();
        if !taken.is_empty() {
            log::info!("roadblock {id}: {} of its {count} cars were cops out of view", taken.len());
            self.cars.retain(|car| !taken.contains(&car.key()));
        }
        for mut car in cars {
            car.set_roadblock();
            car.set_roadblock_group(id);
            self.admit(car);
        }
        let mut deployed = 0;
        for slot in placed.iter().filter(|slot| slot.kind != Element::Car) {
            deployed += u32::from(self.place_prop(&mut block, slot, &ground, props.as_deref_mut()));
        }
        block.standing.spike_strips = block.strips.len() as u32;
        log::info!(
            "roadblock {id}: {} layout of {} cars and {} props ({} strips) at ({:.0}, {:.0}), road {:.1} m",
            if spikes { "spike" } else { "plain" },
            count,
            deployed,
            block.strips.len(),
            site.centre.x,
            site.centre.z,
            site.cr_width
        );
        self.roadblocks.stats.deployed += 1;
        self.roadblocks.stats.props_deployed += deployed;
        self.roadblocks.list.push(block);
        self.police.roadblock_placed();
        self.police.roadblock_props(deployed);
        // The radio hears where the spike strip lies (the layout's offset of its last strip), when there is one.
        let spike_offset = layout.slots.iter().rev().find(|slot| slot.kind == Element::Spike).map(|slot| slot.x as i32);
        self.note_speech(crate::audio::WorldEvent::RoadblockBuilt { spike_offset });
        Ok(count)
    }

    /// The site must be clear and the elements on the road: no other vehicle near an element, the player not in the
    /// way, every element within the road's edges (the rewrite's checks; the original has none of them).
    fn check_clear(&self, placed: &[Placed], site: &Site, target: Vec3) -> Result<(), String> {
        let left = Vec2::new(site.road.left.x, site.road.left.z);
        let right = Vec2::new(site.road.right.x, site.road.right.z);
        let width = left.distance(right).max(1e-3);
        let across = (left - right) / width;
        let forward = Vec2::new(site.forward.x, site.forward.z);
        let centre = Vec2::new(site.centre.x, site.centre.z);
        for slot in placed {
            let at = Vec2::new(slot.position.x, slot.position.z);
            let lateral = (at - right).dot(across);
            if lateral < -EDGE_SLACK || lateral > width + EDGE_SLACK {
                return Err("the layout does not lie on the road".into());
            }
            if (at - centre).dot(forward).abs() > ALONG_LIMIT {
                return Err("the layout runs off the site".into());
            }
            let here = Vec3::new(slot.position.x, 0.0, slot.position.z);
            let near = |p: Vec3, limit: f32| (p.with_y(0.0) - here).length() < limit;
            if near(target, PLAYER_CLEARANCE) {
                return Err("the player is in the way".into());
            }
            if self.cars.iter().any(|car| car.is_finite() && near(car.physics_position(), CLEARANCE)) {
                return Err("another vehicle stands on the site".into());
            }
        }
        Ok(())
    }

    /// Moves the car slots apart by the size of the model each will get, and out of the walls around them
    /// (`spacing.rs`); `models` follows the car slots.
    fn space_cars(&self, placed: &mut [Placed], models: &[usize], collision: &CollisionWorld) {
        let mut slots: Vec<&mut Placed> = placed.iter_mut().filter(|slot| slot.kind == Element::Car).collect();
        let mut footprints: Vec<spacing::Footprint> = slots
            .iter()
            .zip(models)
            .map(|(slot, &model)| {
                let half = self.cop_models[model].model.physics.spec.dimension;
                spacing::Footprint {
                    centre: Vec2::new(slot.position.x, slot.position.z),
                    facing: Vec2::new(slot.heading.cos(), slot.heading.sin()),
                    half_length: half.z,
                    half_width: half.x,
                }
            })
            .collect();
        let height = slots.iter().map(|slot| slot.position.y).fold(f32::NEG_INFINITY, f32::max) + WALL_PROBE_HEIGHT;
        let probe = |from: Vec2, to: Vec2| wall_between(collision, from, to, height);
        let apply = |footprints: &mut [spacing::Footprint], centres: Vec<Vec2>| {
            for (footprint, centre) in footprints.iter_mut().zip(centres) {
                footprint.centre = centre;
            }
        };
        let centres = spacing::separate(&footprints);
        apply(&mut footprints, centres);
        let centres = spacing::clear_walls(&footprints, &probe);
        apply(&mut footprints, centres);
        let centres = spacing::separate(&footprints);
        apply(&mut footprints, centres);
        for (slot, footprint) in slots.iter_mut().zip(&footprints) {
            slot.position.x = footprint.centre.x;
            slot.position.z = footprint.centre.y;
        }
    }

    /// One roadblock car at `slot`: a stopped cop car of cop model `model` facing the layout's heading.
    fn roadblock_car(&mut self, model: usize, slot: &Placed, ground: &WorldGround<'_>) -> Option<AiCar> {
        let at = slot.position;
        let direction = Vec3::new(slot.heading.cos(), 0.0, slot.heading.sin());
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, at, direction, false) {
            log::warn!("roadblock: no road at ({:.0}, {:.0})", at.x, at.z);
            return None;
        }
        let render_forward = space::to_render(direction.to_array());
        // Ground placement resolves the elevation; the layout's lateral and longitudinal offsets are kept.
        let render_at = space::to_render(at.to_array());
        let spawn = Spawn { position: render_at, heading: render_forward.y.atan2(render_forward.x) };
        let start = Start {
            stagger: 0,
            speed: 0.0,
            cruise: Cruise { street: 0.0, highway: 0.0, lateral_g: 1.6, limit_acceleration: false },
            patrol: false,
        };
        let model = &self.cop_models[model].model;
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        let mut car = AiCar::place(model, Role::Traffic, nav, spawn, start, ground)?;
        // A car stands on its wheels as a reset puts it (spec 2.5); without four contacts it keeps the centre seat.
        if !car.seat_on_wheels(ground) {
            log::debug!("roadblock: a car at ({:.0}, {:.0}) keeps its centre seat", at.x, at.z);
        }
        Some(car)
    }

    /// A sawhorse or a spike strip at `slot`: on the surface nearest its point, tilted to its normal, its origin lifted so
    /// the bottom of its collision box meets the surface (spec 2.5). With no surface there it is left out but still counts
    /// as deployed, as the original's prop is created and then fails to place; with 13 props already standing it is not
    /// created at all. Returns whether it counts as deployed.
    fn place_prop(
        &mut self,
        block: &mut Roadblock,
        slot: &Placed,
        ground: &WorldGround<'_>,
        props: Option<&mut PropWorld>,
    ) -> bool {
        let standing: usize = self.roadblocks.list.iter().map(|b| b.strips.len() + b.sawhorses.len()).sum();
        if standing + block.strips.len() + block.sawhorses.len() >= MAX_PLACEABLES {
            log::info!("roadblock: {MAX_PLACEABLES} props already stand, one is left out");
            return false;
        }
        let Some((height, normal)) = surface_near(ground, slot.position) else {
            log::info!("roadblock: no ground under a prop at ({:.0}, {:.0})", slot.position.x, slot.position.z);
            return true;
        };
        let physics = self.roadblocks.physics.as_ref();
        // The strip's long axis lies across its facing; a sawhorse is short both ways.
        let reach = match slot.kind {
            Element::Spike => physics.and_then(|p| p.strip_half).map_or([0.4, 3.2], |half| [half[0], half[1]]),
            _ => [0.4, 1.0],
        };
        let centre = Vec3::new(slot.position.x, height, slot.position.z);
        let (height, normal) = fitted_surface(ground, centre, slot.heading, reach).unwrap_or((height, normal));
        let lift = match slot.kind {
            Element::Spike => physics.and_then(|p| p.strip_lift).unwrap_or(spikes::HALF_HEIGHT - spikes::CENTRE_LIFT),
            _ => physics.and_then(|p| p.sawhorse_lift).unwrap_or(0.0),
        };
        let at = Vec3::new(slot.position.x, height + lift, slot.position.z);
        let frame: Mat4 = super::props::frame(at, slot.heading, normal);
        match slot.kind {
            Element::Spike => {
                let mut strip = Strip::new(Vec2::new(at.x, at.z), at.y, slot.heading);
                if let Some(half) = physics.and_then(|p| p.strip_half) {
                    strip = strip.with_half(half);
                }
                block.strips.push(strip);
                block.strip_frames.push(frame);
            }
            Element::Barrier => {
                let (Some(kit), Some(physics), Some(props)) =
                    (self.roadblocks.kit.as_ref(), self.roadblocks.physics.as_ref(), props)
                else {
                    return true;
                };
                match kit.place_sawhorse(physics, props, frame, Rc::downgrade(&block.owner)) {
                    Some(id) => block.sawhorses.push(id),
                    None => log::warn!("roadblock: a sawhorse could not be put in the world"),
                }
            }
            Element::Car => {}
        }
        true
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod surface_tests;
