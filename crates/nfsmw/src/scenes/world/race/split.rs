//! Two players on one grid: what the front end hands the world to start a split-screen race, and how the race plan takes
//! the second player. Spec: `docs/specs/split-screen.md` section 3.

use super::RacerPlan;
use super::setup::Opponent;
use crate::i18n::t;
use crate::input::PlayerDevice;
use crate::settings::Transmission;

fn custom_identity(folder: &str) -> String {
    format!("my-cars/{}", folder.to_ascii_uppercase())
}

impl crate::scenes::world::WorldScene {
    /// Install player 2's custom model under a distinct identity, so Stock retains its own rig and physics.
    pub(crate) fn prepare_custom_split(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        setup: &SplitSetup,
    ) -> anyhow::Result<()> {
        use crate::scenes::world::{ai::TrafficModel, drive::CarRig, sun};
        use anyhow::Context;

        if !setup.customs[1] {
            return Ok(());
        }
        let folder = &setup.cars[1];
        self.discover_garage_parts(folder)?;
        let id = setup.custom_ids[1].as_deref().context("Player 2 My Cars slot identity is missing")?;
        let selection = crate::garage::my_cars::load(&self.dir, &self.garage, id, folder)?;
        let entry =
            self.garage.cars.iter().find(|car| car.folder == selection.car).context("Custom car is not installed")?;
        let physics = self.physics.car_with_performance(entry.physics_key(), selection.performance)?;
        let tunings = selection.tuning.normalized()?;
        let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
        let mut model =
            nfsmw_data::car::load_customized_driving(&self.dir, &selection.car, &options, &selection.parts)?;
        if let Some(color) = selection.color {
            model.set_body_color(color)?;
        }
        let traffic = self.traffic.as_mut().context("Split screen needs a road network")?;
        let rig =
            std::rc::Rc::new(CarRig::upload(renderer, self.car_keys.next(), model, self.car_shading, sun::to_sun()));
        traffic.add_model(TrafficModel {
            name: custom_identity(id),
            rig,
            physics,
            tunings,
            tractor: false,
            trailer: None,
            cop_lights: None,
            audio_profile: Default::default(),
            keys: self.car_keys.clone(),
        });
        Ok(())
    }
}

/// The two players of a split-screen race.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitSetup {
    /// The car folder each player chose, player 1 first.
    pub cars: [String; 2],
    /// Each car was selected from My Cars, rather than Stock or a preset list.
    pub customs: [bool; 2],
    pub custom_ids: [Option<String>; 2],
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
            name: t!("coop-player", n = 2),
            name_label: None,
            skill: 1.0,
            bias: 1.0,
            car: Some(match self.customs[1] {
                true => custom_identity(self.custom_ids[1].as_deref().unwrap_or(&self.cars[1])),
                false => self.cars[1].clone(),
            }),
            start: None,
        }
    }

    /// Names the two racers and seats the second player's gearbox.
    pub(super) fn name_players(&self, plan: &mut [RacerPlan]) {
        if let Some(first) = plan.first_mut() {
            first.name = t!("coop-player", n = 1);
        }
        if let Some(second) = plan.get_mut(1) {
            second.name = t!("coop-player", n = 2);
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
            customs: [false; 2],
            custom_ids: [None, None],
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
    fn my_cars_and_stock_use_different_model_identities() {
        let mut setup = setup();
        let stock = setup.second_player().car.unwrap();
        setup.customs[1] = true;
        setup.custom_ids[1] = Some("slot-0001".into());
        let custom = setup.second_player().car.unwrap();
        assert_eq!(stock, "PORSCHE911");
        assert_eq!(custom, "my-cars/SLOT-0001");
        assert_ne!(stock, custom);
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
