//! The remote entities of a client world: other players' cars, traffic, police and roadblock cars, drawn from the
//! session's interpolated snapshots (`ClientSession::remote_entities`) through the car rigs and [`CarPose`]s the world
//! draws its own AI cars with. One rig per car model, loaded the first time a model is seen (the entity's type number
//! is its folder's index in the sorted car folders, `net/catalog.rs`); one render key per entity, reused once it is
//! gone. The helicopter, spike strips and moving props are not drawn yet (their models are the traffic's and the
//! props', which a client does not run).

use std::collections::{BTreeMap, BTreeSet};

use blackbox_gfx::{Instance, RenderBackend};
use nfsmw_net::interp::Drawn;
use nfsmw_sim::{EntityId, EntityKind, EntityView};

use super::super::drive::{CarPose, CarRig};
use super::super::net::catalog::CarCatalog;
use super::super::net::flags::{FLAG_BRAKING, FLAG_HEADLIGHTS, FLAG_ROADBLOCK_CAR};
use super::super::{WorldScene, car_loader};
use super::frame::car_pose;

/// A remote car drawn this frame.
#[derive(Debug, Clone, Copy)]
struct RemoteCar {
    type_id: u16,
    key: u32,
    pose: CarPose,
    lamps: [bool; 2],
}

/// The remote cars' rigs and this frame's poses.
#[derive(Default)]
pub struct RemoteCars {
    catalog: Option<CarCatalog>,
    /// The rig of each car model; `None` for a model that did not load (not tried again).
    rigs: BTreeMap<u16, Option<CarRig>>,
    /// The render key of each entity drawn.
    keys: BTreeMap<EntityId, u32>,
    free_keys: Vec<u32>,
    drawn: Vec<RemoteCar>,
}

impl RemoteCars {
    /// Frees every rig.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for rig in std::mem::take(&mut self.rigs).into_values().flatten() {
            rig.release(renderer);
        }
        self.drawn.clear();
    }
}

/// Whether an entity is a car the client draws with a car rig.
pub fn is_car(view: &EntityView) -> bool {
    match view.kind {
        EntityKind::PlayerCar | EntityKind::TrafficCar | EntityKind::PoliceCar => true,
        EntityKind::Roadblock => view.flags & FLAG_ROADBLOCK_CAR != 0,
        EntityKind::Helicopter | EntityKind::MovingProp => false,
    }
}

impl WorldScene {
    /// This frame's remote cars from the session's display list: rigs loaded for new models, keys for new entities,
    /// keys of the entities that went given back.
    pub(super) fn update_remote_cars(&mut self, renderer: &mut dyn RenderBackend) {
        let Some(mode) = self.net_client.as_mut() else { return };
        let remote = &mut mode.remote;
        let catalog = remote.catalog.get_or_insert_with(|| CarCatalog::new(nfsmw_data::car::list(&self.dir)));
        remote.drawn.clear();
        let mut seen = BTreeSet::new();
        for (id, drawn) in mode.display.iter().filter(|(_, drawn)| is_car(&drawn.view) && drawn.fade > 0.0) {
            let type_id = drawn.view.type_id;
            if !remote.rigs.contains_key(&type_id) {
                let rig = load_rig(renderer, &self.dir, catalog, type_id, self.car_shading, self.sun.to_sun());
                remote.rigs.insert(type_id, rig);
            }
            if remote.rigs.get(&type_id).is_none_or(Option::is_none) {
                continue;
            }
            let key = match remote.keys.get(id) {
                Some(&key) => key,
                None => {
                    let key = remote.free_keys.pop().unwrap_or_else(|| self.car_keys.next());
                    remote.keys.insert(*id, key);
                    key
                }
            };
            seen.insert(*id);
            remote.drawn.push(remote_car(type_id, key, drawn));
        }
        let gone: Vec<EntityId> = remote.keys.keys().filter(|id| !seen.contains(id)).copied().collect();
        for id in gone {
            if let Some(key) = remote.keys.remove(&id) {
                remote.free_keys.push(key);
            }
        }
    }

    /// The remote cars' instances for the frame's view.
    pub(in crate::scenes::world) fn remote_car_instances(&self, out: &mut Vec<Instance>) {
        let Some(mode) = self.net_client.as_ref() else { return };
        let remote = &mode.remote;
        let shown = BTreeSet::new();
        for car in &remote.drawn {
            let Some(Some(rig)) = remote.rigs.get(&car.type_id) else { continue };
            rig.instances_lit(car.key, &car.pose, None, &shown, car.lamps, out);
        }
    }
}

fn remote_car(type_id: u16, key: u32, drawn: &Drawn) -> RemoteCar {
    let view = &drawn.view;
    let lamps = [view.flags & FLAG_HEADLIGHTS != 0, view.flags & FLAG_BRAKING != 0];
    RemoteCar { type_id, key, pose: car_pose(view.position, view.orientation), lamps }
}

/// Loads and uploads the model of car type number `type_id`; `None` (logged once) when the install has no such car or
/// it does not load.
fn load_rig(
    renderer: &mut dyn RenderBackend,
    dir: &game_install::GameDir,
    catalog: &CarCatalog,
    type_id: u16,
    shading: crate::settings::CarShading,
    to_sun: glam::Vec3,
) -> Option<CarRig> {
    let Some(folder) = catalog.folder_at(type_id) else {
        log::warn!("net: a remote car of type {type_id}, which this install does not have, is not drawn");
        return None;
    };
    match car_loader::load_named_car(dir, folder) {
        Ok((_, model)) => Some(CarRig::upload(renderer, 0, model, shading, to_sun)),
        Err(error) => {
            log::warn!("net: the remote car {folder} did not load: {error:#}");
            None
        }
    }
}
