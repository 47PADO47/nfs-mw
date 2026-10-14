//! The traffic world's side of the police: the per-frame update that reads the cars, the roads, the zones and the
//! formation, and the helicopter's minimap blip. Child of `police`, so it reads the police state directly.

use super::busted::bust_speed;
use super::*;
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

    /// Gives the helicopter the track's height sheet (read once when the track opens).
    pub fn set_heli_sheet(&mut self, sheet: HeliSheet) {
        self.police.set_heli_sheet(sheet);
    }

    pub fn update_police(&mut self, dt: f32, player: &PlayerView, clear: &dyn Fn(Vec3, Vec3) -> bool) {
        let pursuing = self.is_pursuing();
        let level = self.police.level().unwrap_or(1);
        let at = Vec3::from_array(space::to_physics(player.position));
        let sensed = Sensed {
            seen: self.sighting(level).seen,
            unit_random: self.rng.next_f32(),
            on_road: self.near_road(at),
            nearest_cop: self.nearest_cop(at),
            hit_traffic: self.sighting(level).recent_hit,
            hidden: self.hidden_at(at),
            formation_random: [self.rng.next_f32(), self.rng.next_f32()],
            formation_gap: self.formation_gap(at, player.forward),
            support_random: self.rng.next_f32(),
        };
        // The busted timer's inputs. `flashing` stays false until the post-reset blink exists (FairBusts, E12).
        let bust = BustInput {
            cop_distance: self.nearest_matched_cop(at),
            seen: sensed.seen,
            player_speed: player.speed.abs(),
            slow_speed: bust_speed(level),
            flashing: false,
        };
        self.police.set_rig(self.helicopter_rig.clone());
        let evaded = self.police.update_in_world(dt, pursuing, player, sensed, clear);
        if evaded {
            log::info!("the player got away: the cops give up");
            self.stop_pursuit();
            return;
        }
        // The arrest scene (E8f) hangs off this: the pursuit ends busted and `is_busted` stays set until the next chase.
        if self.police.tick_busted(dt, pursuing, bust) {
            log::info!("the player is busted: the pursuit ends");
            self.stop_pursuit();
        }
    }

    /// Whether a physics-space point is inside a hiding spot: in a HIDDEN zone, at its height (`HIDDEN_HEIGHT`).
    fn hidden_at(&self, at: Vec3) -> bool {
        let zone_point = to_zone_space(at);
        self.police.hidden_zones.iter().any(|z| z.contains(zone_point) && (at.y - z.elevation).abs() < HIDDEN_HEIGHT)
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
    pub fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        let pursuit = self.pursuit.as_ref()?;
        let heat = self.police.level().unwrap_or(pursuit.heat);
        let progress = self.police.evade.progress();
        let limit = blackbox_pursuit::evade::timeout(heat);
        Some(crate::audio::PursuitFeed {
            heat,
            intensity: chase_intensity(self.police.seen, progress, limit),
            secs: self.police.pursuit_time,
            secs_to_evade: (limit - progress).max(0.0),
            player_mph: self.player_speed / super::super::traffic::MPH,
        })
    }

    /// The horizontal distance from a physics-space point to the nearest cop on its level; infinite with none. A cop
    /// on another level (a deck above or a road below) does not count as near.
    fn nearest_cop(&self, at: Vec3) -> f32 {
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| rules::on_level(car.avoidable().position.y - at.y))
            .map(|car| horizontal(car.avoidable().position - at).length())
            .fold(f32::INFINITY, f32::min)
    }

    /// The strength of the speed streaks, 0 to 1.
    pub fn streak_strength(&self) -> f32 {
        self.police.effects().streaks
    }

    /// The extra field of view the high-speed effects add (degrees).
    pub fn fov_kick(&self) -> f32 {
        self.police.effects().fov_kick_deg
    }

    /// The helicopter's minimap blip, while it flies.
    pub(in crate::scenes::world::ai) fn helicopter_blips(&self, pursuing: bool, out: &mut Vec<crate::hud::MapBlip>) {
        let Some((position, heading)) = self.police.helicopter_blip() else { return };
        out.push(crate::hud::MapBlip { style: crate::hud::HELICOPTER, position, heading, active: pursuing });
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
