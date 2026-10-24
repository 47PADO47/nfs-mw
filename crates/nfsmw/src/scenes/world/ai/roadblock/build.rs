//! Building a roadblock (`CreateRoadBlock`, spec sections 2.2 to 2.5): the site, the layout for its width, the cars, the
//! props. A roadblock is built whole or not at all: every car must stand and every element must lie on the road and clear
//! of other vehicles, or the attempt fails and the request tries again on the next manager run.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::roadblock::{self, Element, Layout, NORMAL, Placed, SPIKES};
use blackbox_pursuit::spikes::Strip;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use blackbox_vehicle::Ground;
use glam::{Mat4, Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;
use std::rc::Rc;

use super::site::Site;
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

/// The ground height and the upward normal under a physics-space point, from a ray cast down from just above `at`.
fn ground_under(ground: &WorldGround<'_>, at: Vec3) -> Option<(f32, Vec3)> {
    let origin = Vec3::new(at.x, at.y + 4.0, at.z);
    let hit = ground.hit(origin, Vec3::NEG_Y, 14.0)?;
    let normal = match hit.normal.y < 0.0 {
        true => -hit.normal,
        false => hit.normal,
    };
    Some((origin.y - hit.distance, normal))
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
        let placed = roadblock::place(layout, &site.road);
        self.check_clear(&placed, &site, target)?;
        let ground = WorldGround { collision, surfaces };
        let mut cars = Vec::new();
        let car_slots = placed.iter().filter(|slot| slot.kind == Element::Car).count();
        // The lottery's cars come from cops out of view first (the cop manager's steal rule), then are created.
        let mut thieves = match cause {
            Cause::Lottery => self.steal_candidates(view.position, car_slots),
            Cause::Heavy { .. } | Cause::Console => Vec::new(),
        }
        .into_iter();
        let mut taken = Vec::new();
        for slot in placed.iter().filter(|slot| slot.kind == Element::Car) {
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
            deployed += 1;
            self.place_prop(&mut block, slot, &ground, props.as_deref_mut());
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
        AiCar::place(model, Role::Traffic, nav, spawn, start, ground)
    }

    /// A sawhorse or a spike strip at `slot`: on the ground there, tilted to its normal.
    fn place_prop(
        &mut self,
        block: &mut Roadblock,
        slot: &Placed,
        ground: &WorldGround<'_>,
        props: Option<&mut PropWorld>,
    ) {
        let (height, normal) = ground_under(ground, slot.position).unwrap_or((slot.position.y, Vec3::Y));
        let at = Vec3::new(slot.position.x, height, slot.position.z);
        let frame: Mat4 = super::props::frame(at, slot.heading, normal);
        match slot.kind {
            Element::Spike => {
                let half = self.roadblocks.physics.as_ref().and_then(|p| p.strip_half);
                let mut strip = Strip::new(Vec2::new(at.x, at.z), height, slot.heading);
                if let Some(half) = half {
                    strip = strip.with_half(half);
                }
                block.strips.push(strip);
                block.strip_frames.push(frame);
            }
            Element::Barrier => {
                let (Some(kit), Some(physics), Some(props)) =
                    (self.roadblocks.kit.as_ref(), self.roadblocks.physics.as_ref(), props)
                else {
                    return;
                };
                match kit.place_sawhorse(physics, props, frame, Rc::downgrade(&block.owner)) {
                    Some(id) => block.sawhorses.push(id),
                    None => log::warn!("roadblock: a sawhorse could not be put in the world"),
                }
            }
            Element::Car => {}
        }
    }
}
