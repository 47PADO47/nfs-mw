//! The server's picture of the world after a step: the entity views, which car is whose, the replicate history and
//! encode cache, and the spatial hash that picks each client's relevant entities.

use std::collections::BTreeMap;

use blackbox_net_core::replicate::{EncodeCache, History, SpatialHash};
use nfsmw_net::modes::CarSample;
use nfsmw_net::schema::{GameSchema, Record};
use nfsmw_sim::{EntityKind, EntityView, WorldSim};

/// Ticks of entity frames kept: twice the 64 views a delta may be based on.
const HISTORY_TICKS: usize = 128;
/// The spatial hash's cell, metres.
const CELL_METRES: f32 = 64.0;
/// Entities nearer than this get the full priority weight.
const FULL_WEIGHT_METRES: f32 = 100.0;
/// The lowest weight a relevant entity gets.
const MIN_WEIGHT: f32 = 0.05;

/// The world as the last step left it.
#[derive(Debug)]
pub(crate) struct WorldView {
    views: Vec<EntityView>,
    /// Slot to index in `views` of the slot's car.
    cars: BTreeMap<u8, usize>,
    pub(crate) history: History,
    pub(crate) cache: EncodeCache,
    spatial: SpatialHash,
    radius: f32,
    /// The ids in the newest frame, ascending.
    framed: Vec<u16>,
    record: Record,
    pub(crate) encode_errors: u64,
}

impl WorldView {
    pub(crate) fn new(radius: f32) -> Self {
        Self {
            views: Vec::new(),
            cars: BTreeMap::new(),
            history: History::new(HISTORY_TICKS),
            cache: EncodeCache::new(),
            spatial: SpatialHash::new(CELL_METRES),
            radius,
            framed: Vec::new(),
            record: Record::default(),
            encode_errors: 0,
        }
    }

    /// Reads every entity of `world` and which slot drives which car.
    pub(crate) fn capture(&mut self, world: &impl WorldSim) {
        self.views.clear();
        world.entities(&mut self.views);
        self.cars.clear();
        for (index, view) in self.views.iter().enumerate() {
            if view.kind != EntityKind::PlayerCar {
                continue;
            }
            if let Some(owner) = view.owner {
                self.cars.insert(owner.0, index);
            }
        }
    }

    /// The entity id of `slot`'s car.
    pub(crate) fn car_of(&self, slot: u8) -> Option<u16> {
        self.car_view(slot).map(|view| view.id)
    }

    /// `slot`'s car.
    pub(crate) fn car_view(&self, slot: u8) -> Option<&EntityView> {
        self.views.get(*self.cars.get(&slot)?)
    }

    /// Forgets `slot`'s car until the next capture.
    pub(crate) fn forget_car(&mut self, slot: u8) {
        self.cars.remove(&slot);
    }

    /// One sample per player car, for the mode rules.
    pub(crate) fn car_samples(&self) -> Vec<CarSample> {
        self.cars
            .values()
            .filter_map(|index| self.views.get(*index))
            .filter_map(|view| {
                Some(CarSample {
                    player: view.owner?,
                    entity: view.id,
                    position: view.position,
                    velocity: view.linear_velocity,
                    ..CarSample::default()
                })
            })
            .collect()
    }

    /// Quantises the views into the history's frame of `tick`, resets the encode cache to it and fills the spatial
    /// hash. An entity that does not encode (outside the world bounds, not finite) is left out and counted.
    pub(crate) fn build_frame(&mut self, schema: &GameSchema, tick: u32) {
        let frame = self.history.begin(tick);
        self.cache.reset(tick);
        self.spatial.clear();
        self.framed.clear();
        for view in &self.views {
            if schema.encode(view, None, &mut self.record).is_err() {
                self.encode_errors += 1;
                continue;
            }
            if frame.push(schema.schema(), view.id, self.record.kind(), self.record.values()).is_err() {
                self.encode_errors += 1;
                continue;
            }
            self.framed.push(view.id);
            self.spatial.insert(view.id, view.position[0], view.position[2]);
        }
    }

    /// The entities relevant to a client looking from `focus` (its car), sorted by id with priority weights: every
    /// player car, and everything within the relevancy radius. Without a focus (a spectator) everything is relevant.
    pub(crate) fn candidates(&self, focus: Option<[f32; 3]>, out: &mut Vec<(u16, f32)>) {
        out.clear();
        let Some(focus) = focus else {
            out.extend(self.framed.iter().map(|id| (*id, 1.0)));
            return;
        };
        let mut near = Vec::new();
        self.spatial.query(focus[0], focus[2], self.radius, &mut near);
        for id in &self.framed {
            let Some(view) = self.view(*id) else {
                continue;
            };
            let player = view.kind == EntityKind::PlayerCar;
            if !player && near.binary_search(id).is_err() {
                continue;
            }
            out.push((*id, weight(focus, view.position, player)));
        }
    }

    /// Every entity of the newest frame with weight 1: the replay's view.
    pub(crate) fn everything(&self, out: &mut Vec<(u16, f32)>) {
        out.clear();
        out.extend(self.framed.iter().map(|id| (*id, 1.0)));
    }

    /// Entities in the newest frame.
    pub(crate) fn framed_len(&self) -> usize {
        self.framed.len()
    }

    fn view(&self, id: u16) -> Option<&EntityView> {
        let index = self.views.binary_search_by_key(&id, |view| view.id).ok()?;
        self.views.get(index)
    }
}

/// The priority weight of an entity at `position` for a client at `focus`: 1 for player cars and anything within
/// [`FULL_WEIGHT_METRES`], falling off with distance beyond.
fn weight(focus: [f32; 3], position: [f32; 3], player: bool) -> f32 {
    if player {
        return 1.0;
    }
    let dx = position[0] - focus[0];
    let dz = position[2] - focus[2];
    let distance = (dx * dx + dz * dz).sqrt();
    if !distance.is_finite() || distance <= FULL_WEIGHT_METRES {
        return 1.0;
    }
    (FULL_WEIGHT_METRES / distance).max(MIN_WEIGHT)
}
