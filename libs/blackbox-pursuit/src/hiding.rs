//! Hiding spots as the perpetrator sees them (spec `docs/specs/pursuit-hiding.md`): the zone latch that decides when a
//! spot hides the player, which pursuers it hides the player from, and how far the latch has filled for the HUD.
//!
//! The caller measures the player each update (which zones contain its feet, and whether the cops are still on it) and
//! applies the result; nothing here reads a world or a clock.

/// Seconds in a spot before it hides a player that the cops had lost when it went in.
pub const QUICK_LATCH: f32 = 0.05;
/// The latch of a player that the cops still saw when it went in: the spot cannot hide it until it leaves and goes
/// back in, once the cops have lost it.
pub const SEEN_LATCH: f32 = 99_999.0;
/// The evade level below which the player counts as seen right now when it enters a spot.
pub const SEEN_BELOW: f32 = 0.05;
/// A zone with an elevation only counts when the player's feet are within this many metres of it.
pub const HEIGHT_TOLERANCE: f32 = 1.25;
/// The timer must pass this before the HUD shows the latch filling.
const SHOW_AFTER: f32 = 0.07;

/// Which pursuers a spot hides the player from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Ground cars only.
    Cars,
    /// The helicopter only.
    Helicopters,
    /// Both: the case of every installed spot.
    Both,
}

/// Whether a zone with `elevation` contains a player whose feet are at `feet` (both heights in metres). A zone with no
/// elevation holds the player at any height.
pub fn at_height(elevation: f32, feet: f32) -> bool {
    elevation == 0.0 || (feet - elevation).abs() < HEIGHT_TOLERANCE
}

/// Whether the player counts as unseen when it enters a spot: the evade level has reached [`SEEN_BELOW`]. With no
/// pursuit the player is unseen.
pub fn unseen_on_entry(evade_level: Option<f32>) -> bool {
    evade_level.is_none_or(|level| level >= SEEN_BELOW)
}

/// The latch state of the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hiding {
    in_spot: bool,
    latch: f32,
    timer: f32,
    from_cars: bool,
    from_helicopters: bool,
}

impl Default for Hiding {
    fn default() -> Self {
        Self { in_spot: false, latch: QUICK_LATCH, timer: 0.0, from_cars: false, from_helicopters: false }
    }
}

impl Hiding {
    /// One update of `dt` seconds. `spots` are the scopes of the zones that contain the player's feet, in file order;
    /// `unseen` is [`unseen_on_entry`] of the pursuit's evade level, read when the player goes into a spot.
    ///
    /// The flags are recomputed every update. Time counts once per update however many zones overlap.
    pub fn update(&mut self, dt: f32, spots: &[Scope], unseen: bool) {
        self.from_cars = false;
        self.from_helicopters = false;
        if spots.is_empty() {
            self.in_spot = false;
            self.timer = 0.0;
            return;
        }
        let mut dt = dt;
        for scope in spots {
            match (self.in_spot, self.latch > 999.0) {
                (false, _) => {
                    self.in_spot = true;
                    self.latch = if unseen { QUICK_LATCH } else { SEEN_LATCH };
                }
                (true, true) => self.timer = 0.0,
                (true, false) => self.timer += dt,
            }
            dt = 0.0;
            if self.timer <= self.latch {
                continue;
            }
            match scope {
                Scope::Cars => self.from_cars = true,
                Scope::Helicopters => self.from_helicopters = true,
                Scope::Both => {
                    self.from_cars = true;
                    self.from_helicopters = true;
                }
            }
        }
    }

    /// Hidden from the ground cars.
    pub fn from_cars(&self) -> bool {
        self.from_cars
    }

    /// Hidden from the helicopter.
    pub fn from_helicopters(&self) -> bool {
        self.from_helicopters
    }

    /// Hidden from either: the evade progress runs faster while this holds and no cop has the player in sight.
    pub fn hidden(&self) -> bool {
        self.from_cars || self.from_helicopters
    }

    /// How far the latch has filled, 0 to 1, once it has started to (`IsPartiallyHidden`); `None` before that, and
    /// for a player that went in while seen, whose latch never fills.
    pub fn how_hidden(&self) -> Option<f32> {
        if self.timer <= SHOW_AFTER {
            return None;
        }
        Some((self.timer / self.latch).min(1.0))
    }

    /// Whether the player is in a spot at all (the spot may not hide it yet).
    pub fn in_spot(&self) -> bool {
        self.in_spot
    }

    /// Forgets the state: a new chase, a reset or a teleport.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(hiding: &mut Hiding, seconds: f32, spots: &[Scope], unseen: bool) {
        for _ in 0..(seconds / DT).round() as usize {
            hiding.update(DT, spots, unseen);
        }
    }

    #[test]
    fn a_spot_hides_a_lost_player_after_the_quick_latch() {
        let mut hiding = Hiding::default();
        hiding.update(DT, &[Scope::Both], true);
        assert!(!hiding.hidden(), "the entry update only starts the latch");
        run(&mut hiding, 0.03, &[Scope::Both], true);
        assert!(!hiding.hidden(), "0.05 s have not passed");
        run(&mut hiding, 0.05, &[Scope::Both], true);
        assert!(hiding.from_cars() && hiding.from_helicopters());
    }

    #[test]
    fn a_spot_entered_in_sight_never_hides_the_player_until_it_goes_back_in() {
        let mut hiding = Hiding::default();
        run(&mut hiding, 5.0, &[Scope::Both], false);
        assert!(!hiding.hidden(), "entered while seen: the latch is effectively infinite");
        assert_eq!(hiding.how_hidden(), None);
        // The cops lose the player inside the spot: still not hidden.
        run(&mut hiding, 5.0, &[Scope::Both], true);
        assert!(!hiding.hidden());
        // Leaving and entering again, unseen, works.
        hiding.update(DT, &[], true);
        assert!(!hiding.in_spot());
        run(&mut hiding, 0.2, &[Scope::Both], true);
        assert!(hiding.hidden());
    }

    #[test]
    fn leaving_clears_the_flags_at_once() {
        let mut hiding = Hiding::default();
        run(&mut hiding, 0.5, &[Scope::Both], true);
        assert!(hiding.hidden());
        hiding.update(DT, &[], true);
        assert!(!hiding.hidden());
        assert_eq!(hiding.how_hidden(), None);
    }

    #[test]
    fn the_scope_picks_what_the_spot_hides_the_player_from() {
        let mut cars = Hiding::default();
        run(&mut cars, 0.5, &[Scope::Cars], true);
        assert!(cars.from_cars() && !cars.from_helicopters());
        let mut heli = Hiding::default();
        run(&mut heli, 0.5, &[Scope::Helicopters], true);
        assert!(!heli.from_cars() && heli.from_helicopters());
        assert!(heli.hidden(), "hidden from either speeds the evade up");
    }

    #[test]
    fn overlapping_spots_count_the_time_once() {
        let mut one = Hiding::default();
        let mut two = Hiding::default();
        run(&mut one, 0.04, &[Scope::Both], true);
        run(&mut two, 0.04, &[Scope::Both, Scope::Both], true);
        assert_eq!(one, two);
    }

    #[test]
    fn the_hud_fill_starts_after_a_moment_and_reaches_one() {
        let mut hiding = Hiding::default();
        hiding.update(DT, &[Scope::Both], true);
        assert_eq!(hiding.how_hidden(), None);
        run(&mut hiding, 0.1, &[Scope::Both], true);
        assert_eq!(hiding.how_hidden(), Some(1.0), "the 0.05 s latch is already full by then");
    }

    #[test]
    fn the_entry_reads_the_evade_level() {
        assert!(unseen_on_entry(None));
        assert!(unseen_on_entry(Some(0.05)));
        assert!(!unseen_on_entry(Some(0.0)));
        assert!(!unseen_on_entry(Some(0.049)));
    }

    #[test]
    fn height_counts_from_the_feet_and_a_zero_elevation_holds_any_height() {
        assert!(at_height(106.5, 107.2));
        assert!(!at_height(106.5, 108.0));
        assert!(at_height(0.0, 500.0));
    }
}
