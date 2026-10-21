//! Cars hitting cars, and cops hitting walls, as the chase radio hears them (`SoundAI::OnCollision`): the player's car
//! against a cop or a traffic car, a cop against a traffic car or a wall. Recorded in `speech_events`' log with the
//! physical numbers; the radio applies the speech tuning.

use blackbox_pursuit::speech::RoadblockHit;
use blackbox_pursuit::speech::impact::classify;

use crate::audio::{Crash, WorldEvent};
use crate::scenes::world::ai::TrafficWorld;
use crate::scenes::world::ai::collide::DriverClass;
use crate::scenes::world::drive::PairContact;
use crate::scenes::world::drive::walls::ContactKind;

/// A wall hit that took less than this much speed from a cop (metres per second) is not logged.
const MIN_HIT: f32 = 0.5;

impl TrafficWorld {
    /// A car (`owner`, the one that was hit) met a car of class `striker`. `by_player` is set when the other car is the
    /// player's. The hit is recorded once, from the side of the car that is hit.
    pub(in crate::scenes::world::ai) fn note_pair_hit(
        &mut self,
        owner: usize,
        by_player: bool,
        striker: DriverClass,
        pair: &PairContact,
    ) {
        let car = &self.cars[owner];
        let cop = car.driver_class() == DriverClass::Cop;
        match (by_player, cop) {
            (true, true) => self.note_player_hit_cop(owner, pair),
            (true, false) => {
                let crash = Crash {
                    delta_v: pair.total_delta_v(),
                    closing: pair.closing,
                    speed_before: pair.first_speed(),
                    speed_after: pair.first_speed_after(),
                };
                let semi = car.is_semi();
                self.note_speech(WorldEvent::PlayerHitTraffic { crash, semi });
            }
            (false, true) if striker == DriverClass::Traffic => {
                let cop = car.speech_id();
                self.note_speech(WorldEvent::CopHitTraffic { cop, closing: pair.closing });
            }
            _ => {}
        }
    }

    fn note_player_hit_cop(&mut self, owner: usize, pair: &PairContact) {
        let car = &self.cars[owner];
        let roadblock = car.is_roadblock();
        let deliberate = !(roadblock || car.is_suv() || car.is_braking());
        let impact = classify(&pair.contact(true));
        let crash = Crash {
            delta_v: pair.total_delta_v(),
            closing: pair.closing,
            speed_before: pair.first_speed(),
            speed_after: pair.first_speed_after(),
        };
        let cop = car.speech_id();
        self.note_speech(WorldEvent::PlayerHitCop { cop, crash, impact, deliberate });
        if roadblock {
            self.note_speech(WorldEvent::RoadblockHit(RoadblockHit::Vehicle));
        }
    }

    /// After the cars stepped: a cop that ran into the static world.
    pub(in crate::scenes::world::ai) fn watch_cops_against_walls(&mut self) {
        let mut hits = Vec::new();
        for car in self.cars.iter_mut().filter(|car| car.driver_class() == DriverClass::Cop) {
            let Some(hit) = car.take_surface_hit().filter(|h| h.delta_v >= MIN_HIT) else { continue };
            if !matches!(hit.kind, ContactKind::Barrier | ContactKind::Face) {
                continue;
            }
            hits.push(WorldEvent::CopHitWorld { cop: car.speech_id(), delta_v: hit.delta_v, closing: hit.closing });
        }
        for event in hits {
            self.note_speech(event);
        }
    }
}
