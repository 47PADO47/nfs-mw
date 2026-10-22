//! Two players on one grid: what the front end hands the world to start a split-screen race, and how the race plan takes
//! the second player. Spec: `docs/specs/split-screen.md` section 3.

use super::RacerPlan;
use super::setup::Opponent;
use crate::input::PlayerDevice;
use crate::settings::Transmission;

/// The two players of a split-screen race.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitSetup {
    /// The car folder each player chose, player 1 first.
    pub cars: [String; 2],
    /// The device each player holds (`input::SplitInput`).
    pub devices: [PlayerDevice; 2],
    /// Each player's gearbox.
    pub transmissions: [Transmission; 2],
}

impl SplitSetup {
    /// The second player as the race's one opponent: a racer in their chosen car on the next grid slot. Their skill and
    /// bias are not used (a person drives them).
    pub(super) fn second_player(&self) -> Opponent {
        Opponent {
            name: "Player 2".into(),
            name_label: None,
            skill: 1.0,
            bias: 1.0,
            car: Some(self.cars[1].clone()),
            start: None,
        }
    }

    /// Names the two racers and seats the second player's gearbox.
    pub(super) fn name_players(&self, plan: &mut [RacerPlan]) {
        if let Some(first) = plan.first_mut() {
            first.name = "Player 1".into();
        }
        if let Some(second) = plan.get_mut(1) {
            second.name = "Player 2".into();
            second.person = Some(self.transmissions[1].is_automatic());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::road::Spawn;

    fn setup() -> SplitSetup {
        SplitSetup {
            cars: ["BMWM3GTR".into(), "PORSCHE911".into()],
            devices: [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows],
            transmissions: [Transmission::Automatic, Transmission::Manual],
        }
    }

    fn plan(count: usize) -> Vec<RacerPlan> {
        (0..count)
            .map(|i| RacerPlan {
                name: format!("racer {i}"),
                name_label: None,
                model: (i > 0).then(|| "x".to_owned()),
                spawn: Spawn { position: glam::Vec3::ZERO, heading: 0.0 },
                skill: 0.5,
                bias: 0.5,
                tried: Vec::new(),
                person: None,
            })
            .collect()
    }

    #[test]
    fn the_second_player_is_the_one_opponent_in_their_own_car() {
        let opponent = setup().second_player();
        assert_eq!(opponent.car.as_deref(), Some("PORSCHE911"));
        assert_eq!(opponent.start, None, "they take the next grid slot");
    }

    #[test]
    fn both_racers_are_named_and_the_second_one_is_driven_by_a_person() {
        let mut racers = plan(2);
        setup().name_players(&mut racers);
        assert_eq!((racers[0].name.as_str(), racers[1].name.as_str()), ("Player 1", "Player 2"));
        assert_eq!(racers[0].person, None, "the first player drives the player's own car");
        assert_eq!(racers[1].person, Some(false), "their gearbox is manual");
    }

    #[test]
    fn a_plan_without_the_second_player_is_left_alone() {
        let mut racers = plan(1);
        setup().name_players(&mut racers);
        assert_eq!(racers.len(), 1);
        assert_eq!(racers[0].name, "Player 1");
    }
}
