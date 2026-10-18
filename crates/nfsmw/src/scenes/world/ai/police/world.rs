//! The traffic world's side of the police: the per-frame update that reads the cars, the roads, the zones and the
//! formation, and the helicopter's minimap blip. Child of `police`, so it reads the police state directly.

use super::*;
use crate::hud::PursuitEndCard;
use blackbox_pursuit::busted::BustInput;

impl super::super::TrafficWorld {
    /// Advances the police one frame: heat, infractions, the helicopter and the speed effects. The cops' sight comes
    /// from the traffic's sighting at the chase's level (or level 1 with no chase).
    /// The cops in position at their formation slot and their average distance to it, with the formation chosen. A cop
    /// is in position within `IN_POSITION` metres of its slot.
    fn formation_gap(&self, at: Vec3, forward: Vec3) -> Option<(u32, f32)> {
        /// Metres from a formation slot at which a cop counts as in position (unconfirmed).
        const IN_POSITION: f32 = 4.0;
        let formation = self.police.last_formation()?;
        let fwd = Vec3::from_array(space::to_physics(forward));
        let cops: Vec<usize> = (0..self.cars.len()).filter(|&i| self.cars[i].is_cop()).collect();
        let positions: Vec<Vec2> = cops
            .iter()
            .map(|&i| {
                let p = self.cars[i].avoidable().position;
                Vec2::new(p.x, p.z)
            })
            .collect();
        let aims = assign(formation, &positions, Vec2::new(at.x, at.z), Vec2::new(fwd.x, fwd.z));
        let gaps: Vec<f32> = positions
            .iter()
            .zip(aims)
            .filter_map(|(p, aim)| aim.map(|a| p.distance(a)))
            .filter(|d| *d <= IN_POSITION)
            .collect();
        if gaps.is_empty() {
            return None;
        }
        Some((gaps.len() as u32, gaps.iter().sum::<f32>() / gaps.len() as f32))
    }

    /// Whether a support roll asked for a roadblock this frame; the world places it and calls `roadblock_placed`.
    pub fn take_roadblock_request(&mut self) -> bool {
        self.police.take_roadblock_request()
    }

    /// The roadblock was placed: no second heavy roadblock this chase.
    pub fn roadblock_placed(&mut self) {
        self.police.roadblock_placed();
    }

    /// The player pressed the reset key: the post-reset blink starts (the reset penalty applies while it runs).
    pub fn note_manual_reset(&mut self) {
        self.spike_patches = None;
        self.police.note_manual_reset();
    }

    /// Gives the helicopter the track's height sheet (read once when the track opens).
    pub fn set_heli_sheet(&mut self, sheet: HeliSheet) {
        self.police.set_heli_sheet(sheet);
    }

    pub fn update_police(&mut self, dt: f32, player: &PlayerView, clear: &dyn Fn(Vec3, Vec3) -> bool) {
        self.apply_mod_requests();
        let pursuing = self.is_pursuing();
        self.police.tick_bust(dt);
        let level = self.police.level().unwrap_or(1);
        let at = Vec3::from_array(space::to_physics(player.position));
        let strikes = self.take_strikes();
        let sensed = Sensed {
            seen: self.sighting(level).seen,
            unit_random: self.rng.next_f32(),
            on_road: self.near_road(at),
            nearest_cop: self.nearest_cop(at),
            hit_traffic: self.sighting(level).recent_hit,
            struck_cop: strikes.cop,
            struck_property: strikes.property,
            hidden: self.hidden_from_cars(at),
            hidden_from_heli: self.hidden_from_helicopters(at),
            in_tunnel: self.in_tunnel(at),
            formation_random: [self.rng.next_f32(), self.rng.next_f32()],
            formation_gap: self.formation_gap(at, player.forward),
            support_random: self.rng.next_f32(),
        };
        // The busted timer's inputs. `flashing` is the post-reset blink, started by a manual reset (FairBusts' penalty).
        let bust = BustInput {
            cop_distance: self.nearest_matched_cop(at),
            seen: self.police.is_perp_in_sight(),
            player_speed: player.speed.abs(),
            slow_speed: self.police.bust_speed(level),
            flashing: self.police.flashing(),
        };
        self.police.set_rig(self.helicopter_rig.clone());
        // Busted first: a busted frame does not tick the escape. `is_busted` stays set until the next chase.
        let busted = self.police.tick_busted(dt, pursuing, bust);
        let evaded = self.police.update_in_world(dt, pursuing, player, sensed, clear);
        self.publish_to_mods();
        if busted {
            log::info!("the player is busted: the pursuit ends");
            self.police.start_bust(self.rng.next_f32());
            self.end_pursuit(EndReason::Busted);
            return;
        }
        if evaded {
            log::info!("the player got away: the cops give up");
            self.end_pursuit(EndReason::Evaded);
        }
    }

    /// How the last chase ended, for the camera and the speech (`EndReason::None` while one runs, and before any chase).
    pub fn end_reason(&self) -> EndReason {
        self.police.end_reason()
    }

    /// The cool-down meter, 0 to 1, for the pursuit board's bar.
    pub fn evade_level(&self) -> f32 {
        self.police.evade_level()
    }

    /// The end-of-pursuit card, while a chase ended busted or evaded. `None` otherwise.
    pub fn pursuit_end_card(&self) -> Option<PursuitEndCard> {
        // A free-roam bust shows its ticket instead of the card (`busted/flow.rs`).
        if self.police.bust.replaces_end_card() {
            return None;
        }
        let reason = self.police.end_reason();
        if reason == EndReason::None {
            return None;
        }
        Some(PursuitEndCard {
            reason,
            cops_destroyed: self.police.cops_destroyed(),
            secs: self.police.pursuit_time,
            since_end: self.police.since_end(),
        })
    }

    /// Whether a physics-space point is in a HIDDEN zone that hides it from cars: any zone but one flagged `Heli`.
    pub(in crate::scenes::world::ai) fn hidden_from_cars(&self, at: Vec3) -> bool {
        self.in_hidden_zone(at, |zone| zone.data[0] as u32 != HELI_HASH)
    }

    /// Whether a physics-space point is in a HIDDEN zone that hides it from helicopters: any zone but one flagged `Car`.
    fn hidden_from_helicopters(&self, at: Vec3) -> bool {
        self.in_hidden_zone(at, |zone| zone.data[0] as u32 != CAR_HASH)
    }

    /// Whether `at` is inside a HIDDEN zone at its height (`HIDDEN_HEIGHT`) that `hides` accepts.
    fn in_hidden_zone(&self, at: Vec3, hides: impl Fn(&TrackZone) -> bool) -> bool {
        let zone_point = to_zone_space(at);
        self.police
            .hidden_zones
            .iter()
            .any(|z| hides(z) && z.contains(zone_point) && (at.y - z.elevation).abs() < HIDDEN_HEIGHT)
    }

    /// Whether a physics-space point is inside a tunnel zone. The zones have no height test: a tunnel is its outline.
    fn in_tunnel(&self, at: Vec3) -> bool {
        let zone_point = to_zone_space(at);
        self.police.tunnel_zones.iter().any(|z| z.contains(zone_point))
    }

    /// Whether a physics-space point is within `ROAD_EDGE` of a road centreline, horizontally.
    fn near_road(&self, at: Vec3) -> bool {
        closest_segment(&self.network, &self.index, at, Vec3::ZERO, 0.0, SegmentFilter::default())
            .is_some_and(|hit| horizontal(hit.point - at).length() <= ROAD_EDGE)
    }

    /// The cop cars wrecked in the chase, for the pursuit board.
    pub fn cops_destroyed(&self) -> u32 {
        self.police.cops_destroyed()
    }

    /// The chase as the audio and the speech read it each frame; `None` with no chase on.
    /// The scene sets `rain_intensity` (the traffic does not hold the weather, so it reads 0 here).
    pub fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        let pursuit = self.pursuit.as_ref()?;
        let heat = self.police.level().unwrap_or(pursuit.heat);
        let limit = blackbox_pursuit::evade::timeout(heat);
        let level = self.police.level().unwrap_or(1);
        Some(crate::audio::PursuitFeed {
            heat,
            intensity: chase_intensity(self.police.seen, self.police.escape_progress(), limit),
            secs: self.police.pursuit_time,
            secs_to_evade: self.police.secs_to_evade(),
            player_mph: self.player_speed / super::super::traffic::MPH,
            heli_present: self.police.helicopter_present(),
            rain_intensity: 0.0,
            roadblock_active: self.police.roadblock_standing(),
            player_hit: self.sighting(level).recent_hit,
            cops_destroyed: self.police.cops_destroyed(),
        })
    }

    /// The horizontal distance from a physics-space point to the nearest cop on its level; infinite with none. A cop
    /// on another level (a deck above or a road below) does not count as near.
    fn nearest_cop(&self, at: Vec3) -> f32 {
        self.cars
            .iter()
            .filter(|car| car.is_live_police())
            .filter(|car| rules::on_level(car.avoidable().position.y - at.y))
            .map(|car| horizontal(car.avoidable().position - at).length())
            .fold(f32::INFINITY, f32::min)
    }

    /// The extra field of view the high-speed effects add (degrees).
    pub fn fov_kick(&self) -> f32 {
        self.police.effects().fov_kick_deg
    }

    /// The radial blur the high-speed effects add, 0 to 1, from the screen centre (`FocusFilter::radial_blur`).
    pub fn radial_blur(&self) -> f32 {
        self.police.effects().blur
    }

    /// The helicopter's minimap blip, while it flies.
    pub(in crate::scenes::world::ai) fn helicopter_blips(&self, pursuing: bool, out: &mut Vec<crate::hud::MapBlip>) {
        let Some(position) = self.police.helicopter_blip() else { return };
        out.push(crate::hud::MapBlip { style: crate::hud::HELICOPTER, position, active: pursuing });
    }
}

/// How tense a chase is, 0 (the cops lost the player) to 1 (they see the player). It is full while the cops see the
/// player, and falls as the unseen progress fills towards the evade timeout.
pub fn chase_intensity(seen: bool, progress: f32, timeout: f32) -> f32 {
    if seen {
        return 1.0;
    }
    (1.0 - progress / timeout.max(f32::EPSILON)).clamp(0.0, 1.0)
}
