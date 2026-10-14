//! What the helicopter does each think: follow the player at a height above it, lunge in for a skid hit, search
//! for a lost player, or leave. Each returns a [`Goal`] for the controller. Spec: `ai-helicopter.md` §6.

use glam::{Vec2, Vec3};

use super::control::Goal;

/// The speed (m/s) the helicopter flies at when it follows, searches or strikes (`drive_speed = 100` for the skid).
pub const STRIKE_SPEED: f32 = 100.0;
/// The speed the search flies at, m/s.
pub const SEARCH_SPEED: f32 = 70.0;
/// Seconds a skid hit's approach lasts before the timer can run out.
pub const SKID_TIMER: f32 = 8.0;
/// The helicopter is within this window of the player (horizontal metres) to start a skid hit.
pub const SKID_WINDOW: (f32, f32) = (5.0, 35.0);
/// Height (m) above the player within which a skid hit can start.
pub const SKID_HEIGHT: f32 = 13.0;
/// Exit: height (m) above the player it climbs to.
pub const EXIT_HEIGHT: f32 = 25.0;

/// The player's car as the helicopter sees it this think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    /// The direction the car faces, unit length.
    pub forward: Vec3,
}

impl Player {
    /// The car's right side (the spec's convention: `forward` turned clockwise about +Y).
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.forward.z, 0.0, -self.forward.x)
    }
}

/// Horizontal distance between two points.
pub fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// The follow goal (straight line). `skid_ready` is a skid hit enabled for this level and its cooldown over; it
/// shortens the lead by a quarter. `skid_timer` above 0 means a skid is in progress and holds the height at +2 m.
pub fn follow(heli: Vec3, player: &Player, skid_timer: f32, skid_ready: bool) -> Goal {
    let speed = player.velocity.length();
    let mut lead = (30.0 + 0.4 * speed).min(45.0);
    if skid_ready {
        lead *= 0.75;
    }
    let mut seek = player.position + player.forward * lead;
    let dist = horizontal_distance(heli, player.position);
    let height = if skid_timer > 0.0 {
        2.0
    } else if dist > 55.0 || skid_timer > -5.0 {
        12.0
    } else {
        6.0
    };
    seek.y = player.position.y + height;

    let d = horizontal_distance(heli, seek);
    let to_seek = (seek - heli).normalize_or_zero();
    let mut along = to_seek.dot(player.velocity);
    if along < 0.0 {
        along *= 0.5;
    }
    let close = if d > 15.0 {
        if d > 300.0 { 50.0 } else { 30.0 }
    } else if d > 3.0 {
        20.0
    } else {
        d
    };
    let look_at = if d < 5.0 && speed < 15.0 { player.position } else { seek };
    Goal { drive: seek, speed: close + along, dest_velocity: player.velocity, look_at }
}

/// Whether the helicopter is in the spot where a skid hit can start: in the window, ahead of the player's motion
/// and not too high above it. The spec's rule, horizontal except for the height.
pub fn skid_can_start(heli: Vec3, player: &Player) -> bool {
    let to = heli - player.position;
    let dist = horizontal_distance(heli, player.position);
    let (near, far) = SKID_WINDOW;
    if dist < near || dist > far {
        return false;
    }
    let moving = Vec3::new(player.velocity.x, 0.0, player.velocity.z).normalize_or_zero();
    let ahead = Vec3::new(to.x, 0.0, to.z).dot(moving) / dist > 0.707;
    ahead && to.y < SKID_HEIGHT
}

/// The side the skid hit comes from: +1 or -1, the side the helicopter is already on, so it does not cross over.
pub fn skid_side(heli: Vec3, player: &Player) -> f32 {
    let offset = (heli - player.position).dot(player.right());
    if offset >= 0.0 { 1.0 } else { -1.0 }
}

/// The skid approach goal: slide in 6 m beside the player at its height plus about 2 m.
pub fn skid_approach(heli: Vec3, player: &Player, side: f32, speed: f32) -> Goal {
    let lift = if heli.y - player.position.y < 2.0 { 4.8 } else { 1.8 };
    let seek = player.position + player.right() * (6.0 * side) + player.velocity * 0.23 + Vec3::new(0.0, lift, 0.0);
    Goal { drive: seek, speed, dest_velocity: player.velocity, look_at: player.position + player.velocity }
}

/// Whether the approach has reached its side: close, and well off the player's centreline.
pub fn skid_strikes(heli: Vec3, player: &Player, side: f32) -> bool {
    let seek = player.position + player.right() * (6.0 * side) + player.velocity * 0.23;
    let across = (heli - player.position).dot(player.right()).abs();
    horizontal_distance(heli, seek) < 4.0 && across > 1.9
}

/// The strike: slide sideways into the car at its own height.
pub fn skid_strike(player: &Player, side: f32, speed: f32) -> Goal {
    let offset = player.right() * (6.0 * side);
    let seek = player.position + player.velocity * 0.092 - 0.5 * offset + Vec3::new(0.0, 0.4, 0.0);
    Goal { drive: seek, speed, dest_velocity: player.velocity, look_at: player.position + player.velocity }
}

/// Whether a skid hit is abandoned: the helicopter is ahead and far, below the player, or behind and beyond 12 m.
pub fn skid_aborts(heli: Vec3, player: &Player) -> bool {
    let to = heli - player.position;
    let dist = horizontal_distance(heli, player.position);
    let ahead = to.dot(player.forward) > 0.0;
    (ahead && dist > 40.0) || heli.y < player.position.y || (!ahead && dist > 12.0)
}

/// The search point: `distance` between 0.2 and 0.9 of `radius` from `centre` at `angle` turns, at the floor (if
/// the heli sheet covers it, never below `centre`'s height) plus 5 m. `unit_random` in `0..=1` picks the distance.
pub fn search_point(centre: Vec3, angle_turns: f32, unit_random: f32, radius: f32, floor: Option<f32>) -> Vec3 {
    let dist = unit_random.clamp(0.0, 1.0) * 0.7 * radius + 0.2 * radius;
    let angle = angle_turns * std::f32::consts::TAU;
    let base = match floor {
        Some(f) => centre.y.max(f),
        None => centre.y,
    };
    Vec3::new(centre.x + angle.cos() * dist, base + 5.0, centre.z + angle.sin() * dist)
}

/// The search goal: fly to a search point at search speed, matching the velocity of the helicopter.
pub fn search(point: Vec3, heli_velocity: Vec3, speed: f32) -> Goal {
    Goal { drive: point, speed, dest_velocity: heli_velocity, look_at: point }
}

/// Where the helicopter is in its exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStage {
    /// Climb until 15 m above the player, then head for a point 85 m ahead of it, 25 m up.
    SeekUp,
    /// Dive to a point 200 m behind the player, on one side.
    SeekCar,
    /// Fly out, away from the player.
    FlyOut,
}

/// The exit: the stage, and where it is headed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exit {
    pub stage: ExitStage,
    pub seek: Vec3,
}

impl Exit {
    /// The exit starts by climbing above the helicopter's current place.
    pub fn start(heli: Vec3) -> Self {
        Self { stage: ExitStage::SeekUp, seek: heli + Vec3::new(0.0, EXIT_HEIGHT, 0.0) }
    }

    /// Advances the stage from the helicopter and the player, then returns the goal for this think.
    pub fn think(&mut self, heli: Vec3, heli_velocity: Vec3, player: &Player, speed: f32) -> Goal {
        match self.stage {
            ExitStage::SeekUp if heli.y - player.position.y >= 15.0 => {
                self.seek = player.position + player.forward * 85.0 + Vec3::new(0.0, EXIT_HEIGHT, 0.0);
                self.stage = ExitStage::SeekCar;
            }
            ExitStage::SeekCar if (heli - self.seek).length() < 5.0 => {
                let right = player.right();
                let side = if right.dot(heli_velocity) >= 0.0 { -9.0 } else { 3.0 };
                let direction = player.forward + right * side;
                self.seek = player.position - direction * 200.0 + Vec3::new(0.0, 30.0, 0.0);
                self.stage = ExitStage::FlyOut;
            }
            _ => {}
        }
        Goal { drive: self.seek, speed, dest_velocity: heli_velocity, look_at: self.seek }
    }

    /// The exit is over: the helicopter is well above the player, behind it and far away.
    pub fn finished(&self, heli: Vec3, player: &Player) -> bool {
        let to = heli - player.position;
        self.stage == ExitStage::FlyOut && to.y > 25.0 && to.dot(player.forward) < 0.0 && to.length() > 150.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> Player {
        Player { position: Vec3::ZERO, velocity: Vec3::new(30.0, 0.0, 0.0), forward: Vec3::X }
    }

    #[test]
    fn follow_leads_the_player_and_flies_at_its_pace() {
        let heli = Vec3::new(-100.0, 80.0, 0.0);
        let goal = follow(heli, &player(), -20.0, false);
        assert!((goal.drive.x - 42.0).abs() < 1e-4, "lead is 30 + 0.4*30 = 42 m ahead");
        assert!((goal.drive.y - 12.0).abs() < 1e-4, "12 m above the player when far");
        assert_eq!(goal.dest_velocity, player().velocity);
    }

    #[test]
    fn a_skid_ready_helicopter_leads_a_quarter_less() {
        let heli = Vec3::new(-100.0, 80.0, 0.0);
        let plain = follow(heli, &player(), -20.0, false).drive.x;
        let ready = follow(heli, &player(), -20.0, true).drive.x;
        assert!((ready - (plain * 0.75)).abs() < 1e-3);
    }

    #[test]
    fn a_skid_in_progress_holds_the_height_at_two_metres() {
        let heli = Vec3::new(-100.0, 80.0, 0.0);
        assert!((follow(heli, &player(), 4.0, false).drive.y - 2.0).abs() < 1e-4);
    }

    #[test]
    fn the_follow_speed_steps_with_the_distance_to_the_point() {
        let mut p = player();
        p.velocity = Vec3::ZERO;
        let far = follow(Vec3::new(-400.0, 80.0, 0.0), &p, -20.0, false);
        assert!((far.speed - 50.0).abs() < 1e-4, "far away flies at 50");
        // 12 m short of the point 30 m ahead: between 3 and 15 m.
        let near = follow(Vec3::new(18.0, 80.0, 0.0), &p, -20.0, false);
        assert!((near.speed - 20.0).abs() < 1e-4, "between 3 and 15 m flies at 20");
    }

    #[test]
    fn skid_starts_in_the_window_ahead_and_low() {
        let p = player();
        assert!(skid_can_start(Vec3::new(20.0, 8.0, 0.0), &p), "ahead, 20 m off, 8 m up");
        assert!(!skid_can_start(Vec3::new(-20.0, 8.0, 0.0), &p), "behind the player");
        assert!(!skid_can_start(Vec3::new(20.0, 30.0, 0.0), &p), "too high");
        assert!(!skid_can_start(Vec3::new(2.0, 1.0, 0.0), &p), "too close");
    }

    #[test]
    fn the_skid_side_is_the_side_already_taken() {
        let p = player();
        // The player's right is -Z when it faces +X, so a helicopter at -Z is on the right.
        assert_eq!(skid_side(Vec3::new(0.0, 0.0, -10.0), &p), 1.0);
        assert_eq!(skid_side(Vec3::new(0.0, 0.0, 10.0), &p), -1.0);
    }

    #[test]
    fn the_strike_starts_once_close_and_across_the_car() {
        let p = player();
        let side = 1.0;
        // Just above the approach point, which sits 6 m to the right and 0.23 s of velocity ahead.
        let across = Vec3::new(6.9, 1.0, -6.0);
        assert!(skid_strikes(across, &p, side));
        assert!(!skid_strikes(Vec3::new(0.0, 1.0, 0.0), &p, side), "on the centreline: no strike");
    }

    #[test]
    fn a_skid_aborts_when_far_ahead_or_under_the_car() {
        let p = player();
        assert!(skid_aborts(Vec3::new(60.0, 10.0, 0.0), &p));
        assert!(skid_aborts(Vec3::new(5.0, -1.0, 0.0), &p));
        assert!(!skid_aborts(Vec3::new(10.0, 10.0, 0.0), &p));
    }

    #[test]
    fn search_points_lie_between_two_tenths_and_nine_tenths_of_the_radius() {
        let c = Vec3::ZERO;
        let near = search_point(c, 0.0, 0.0, 1000.0, None);
        let far = search_point(c, 0.0, 1.0, 1000.0, None);
        assert!((near.x - 200.0).abs() < 1e-3);
        assert!((far.x - 900.0).abs() < 1e-3);
        assert!((near.y - 5.0).abs() < 1e-5, "5 m above the centre");
    }

    #[test]
    fn the_search_point_is_never_below_the_centre_on_the_sheet() {
        let c = Vec3::new(0.0, 40.0, 0.0);
        let p = search_point(c, 0.25, 0.5, 1000.0, Some(10.0));
        assert!((p.y - 45.0).abs() < 1e-5);
    }

    #[test]
    fn exit_climbs_then_dives_then_flies_out_and_finishes() {
        let p = player();
        let mut heli = Vec3::new(-20.0, 10.0, 0.0);
        let mut exit = Exit::start(heli);
        assert_eq!(exit.stage, ExitStage::SeekUp);
        heli.y = 30.0;
        exit.think(heli, Vec3::ZERO, &p, STRIKE_SPEED);
        assert_eq!(exit.stage, ExitStage::SeekCar, "15 m above the player: next stage");
        heli = exit.seek;
        exit.think(heli, Vec3::ZERO, &p, STRIKE_SPEED);
        assert_eq!(exit.stage, ExitStage::FlyOut);
        let far_behind = Vec3::new(-300.0, 60.0, 0.0);
        assert!(exit.finished(far_behind, &p));
        assert!(!exit.finished(Vec3::new(-100.0, 60.0, 0.0), &p), "not far enough");
    }
}
