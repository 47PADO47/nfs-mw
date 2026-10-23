//! `WorldSim for ToyWorld`: the tick order, the entity view and the state hash.

use blackbox_vehicle::InputState;

use super::car::{Car, Controls};
use super::player::{PlayerSlot, grid_position};
use super::{FIRST_BOT_ID, MAX_PLAYERS, PLAYER_TOP_SPEED, ToyWorld, finite_or};
use crate::hash::StateHasher;
use crate::{
    EntityId, EntityKind, EntityView, OwnState, PlayerId, PlayerJoin, SimError, SimEvent, SimInputs, Tick, WorldSim,
};

/// Police cars carry the siren bit.
const FLAG_SIREN: u32 = 8;

impl WorldSim for ToyWorld {
    fn tick(&self) -> Tick {
        self.tick
    }

    fn add_player(&mut self, id: PlayerId, join: &PlayerJoin) -> Result<(), SimError> {
        if self.slot_index(id).is_some() {
            return Err(SimError::AlreadyJoined(id));
        }
        if join.car_type >= self.config.car_types {
            return Err(SimError::UnknownCar(join.car_type));
        }
        let Some(index) = self.players.iter().position(Option::is_none) else {
            return Err(SimError::Full);
        };
        let mut pos = grid_position(index);
        if let Some([x, _, z]) = join.spawn {
            pos = finite_or([x, z], pos);
        }
        let power = 1.0 + (join.car_type % 4) as f32 * 0.05;
        let mut car = Car::new(pos, [0.0, 1.0], power, PLAYER_TOP_SPEED);
        car.keep_inside(self.config.bounds);
        car.speed = 0.0;
        let race = super::RaceProgress { start: self.tick, lap_start: self.tick, ..Default::default() };
        let slot = PlayerSlot { id, car_type: join.car_type, car, race, chase: Default::default() };
        self.players[index] = Some(slot);
        Ok(())
    }

    fn remove_player(&mut self, id: PlayerId) {
        let Some(index) = self.slot_index(id) else {
            return;
        };
        self.players[index] = None;
        self.moved.retain(|player| *player != id);
    }

    fn step(&mut self, inputs: &SimInputs, events: &mut Vec<SimEvent>) {
        events.extend(self.moved.drain(..).map(|player| SimEvent::Moved { player }));
        let bounds = self.config.bounds;
        for slot in self.players.iter_mut().flatten() {
            let neutral = InputState::default();
            let input = inputs.get(slot.id).unwrap_or(&neutral);
            slot.car.step(Controls::from_input(input), bounds);
        }
        for bot in &mut self.traffic {
            let controls = bot.wander(&mut self.rng, bounds);
            bot.car.step(controls, bounds);
        }
        let mut police = std::mem::take(&mut self.police);
        for bot in &mut police {
            let quarry = self.nearest_player(bot.car.pos);
            let controls = bot.chase(quarry, &mut self.rng, bounds);
            bot.car.step(controls, bounds);
        }
        self.police = police;
        self.resolve_contacts(events);
        self.update_race(events);
        self.update_chase(events);
        self.tick = self.tick.wrapping_add(1);
    }

    fn entities(&self, out: &mut Vec<EntityView>) {
        for (index, slot) in self.players.iter().enumerate() {
            let Some(slot) = slot else {
                continue;
            };
            out.push(view(index as EntityId, EntityKind::PlayerCar, Some(slot.id), slot.car_type as u16, &slot.car, 0));
        }
        for (index, bot) in self.traffic.iter().enumerate() {
            let id = FIRST_BOT_ID + index as EntityId;
            out.push(view(id, EntityKind::TrafficCar, None, (index % 8) as u16, &bot.car, 0));
        }
        let first_police = FIRST_BOT_ID + self.traffic.len() as EntityId;
        for (index, bot) in self.police.iter().enumerate() {
            out.push(view(first_police + index as EntityId, EntityKind::PoliceCar, None, 0, &bot.car, FLAG_SIREN));
        }
    }

    fn own_state(&self, id: PlayerId) -> Option<OwnState> {
        let car = self.player_car(id)?;
        Some(OwnState {
            rpm: car.rpm(),
            gear: car.gear(),
            nitrous: car.nitrous,
            boost: car.boost,
            flags: car.flags() as u8,
        })
    }

    fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_u32(self.tick);
        h.write_u64(self.rng.state());
        h.write_u8(self.finished);
        h.write_u32(self.moved.len() as u32);
        for player in &self.moved {
            h.write_u8(player.0);
        }
        for slot in &self.players {
            h.write_bool(slot.is_some());
            if let Some(slot) = slot {
                slot.hash_into(&mut h);
            }
        }
        h.write_u32(self.traffic.len() as u32);
        for bot in &self.traffic {
            bot.hash_into(&mut h);
        }
        h.write_u32(self.police.len() as u32);
        for bot in &self.police {
            bot.hash_into(&mut h);
        }
        h.finish()
    }
}

const _: () = assert!(MAX_PLAYERS <= u8::MAX as usize);

fn view(
    id: EntityId,
    kind: EntityKind,
    owner: Option<PlayerId>,
    type_id: u16,
    car: &Car,
    extra_flags: u32,
) -> EntityView {
    let v = car.velocity();
    EntityView {
        id,
        kind,
        owner,
        position: [car.pos[0], 0.0, car.pos[1]],
        orientation: car.orientation(),
        linear_velocity: [v[0], 0.0, v[1]],
        angular_velocity: [0.0, car.yaw_rate, 0.0],
        type_id,
        flags: car.flags() | extra_flags,
    }
}
