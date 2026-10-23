//! One world step: each driver's input of the tick from its window, [`nfsmw_sim::WorldSim::step`], the new picture
//! of the world, correction causes for the own block, and the round's rules.

use blackbox_net_core::session::Phase;
use nfsmw_net::input::InputSource;
use nfsmw_net::schema::CorrectionCause;
use nfsmw_sim::{EntityId, PlayerId, SimEvent};

use super::Server;
use crate::control::WorldControl;

impl<W: WorldControl> Server<W> {
    /// Steps the world one tick.
    pub(super) fn step_world(&mut self) {
        let tick = self.world.tick();
        let running = self.lobby.phase() == Phase::Running && self.race.start_tick.is_none_or(|start| tick >= start);
        self.inputs.clear();
        for (&slot, peer) in &self.slots {
            if self.frames.car_of(slot).is_none() {
                continue;
            }
            let Some(state) = self.peers.get_mut(peer) else {
                continue;
            };
            let (code, source) = state.window.input_for(tick);
            let received = source == InputSource::Received;
            state.inputs.simulated(tick, received);
            if received {
                state.last_input = Some(tick);
            }
            if running {
                self.inputs.set(PlayerId(slot), code.to_input());
            }
        }
        let mut events = Vec::new();
        self.world.step(&self.inputs, &mut events);
        self.frames.capture(&self.world);
        self.note_causes(&events);
        self.feed_round(&events);
        self.handle_mode_events();
    }

    /// Marks the own block of a player whose car the server moved: a reset or teleport, or a contact.
    fn note_causes(&mut self, events: &[SimEvent]) {
        for event in events {
            match *event {
                SimEvent::Moved { player } => self.set_cause(player.0, CorrectionCause::Teleport),
                SimEvent::Contact { a, b, .. } => {
                    for entity in [a, b] {
                        if let Some(slot) = self.slot_of_entity(entity) {
                            self.set_cause(slot, CorrectionCause::Contact);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn slot_of_entity(&self, entity: EntityId) -> Option<u8> {
        self.race.roster.player_of(entity).map(|player| player.0)
    }

    /// Sets the cause of `slot`'s next own block unless one is set already.
    fn set_cause(&mut self, slot: u8, cause: CorrectionCause) {
        let Some(peer) = self.slots.get(&slot).and_then(|peer| self.peers.get_mut(peer)) else {
            return;
        };
        if peer.cause == CorrectionCause::None {
            peer.cause = cause;
        }
    }
}
