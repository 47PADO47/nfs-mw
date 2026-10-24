//! Pursuit state follows race lifetime rather than the availability of `WorldScene::race` during a frame.

use nfsmw_data::gameplay::{Race, RaceType};

use crate::scenes::world::ai::PursuitContext;

pub(super) fn context_of(race: &Race, career: bool) -> PursuitContext {
    PursuitContext {
        racing: true,
        race_heat_active: false,
        pursuit_event: race.params.pursuit_race,
        career_event: career,
        challenge: race.race_type == Some(RaceType::Challenge),
        final_pursuit: race.name == "race_bin_01/epic_pursuit",
        event_heat: race.params.pursuit_heat,
    }
}

impl crate::scenes::world::WorldScene {
    pub(super) fn sync_race_pursuit_context(&mut self, session: &super::RaceSession) {
        let mut context = context_of(&session.origin.race, session.options.career);
        context.race_heat_active = session.stage == super::Stage::Running
            && session.engine.as_ref().is_none_or(|engine| engine.phase() == blackbox_race::Phase::Racing);
        if let Err(why) = crate::scenes::world::ai::configure_pursuit_context(self, context) {
            log::warn!("race: could not select pursuit context: {why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forced_special_heat_does_not_identify_the_final_event() {
        let mut context =
            PursuitContext { racing: true, race_heat_active: true, challenge: true, ..Default::default() };
        context.event_heat.force_heat = 6.0;
        assert!(!context.final_pursuit);
        assert_eq!(context.table(), nfsmw_data::pursuit::PursuitTable::Race);
    }
}
