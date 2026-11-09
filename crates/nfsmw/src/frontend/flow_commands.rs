//! Carries out the commands the screens ask for: the one dispatch from `Command` to the front end's flows.

use crate::app::Host;
use crate::frontend::logic::{Command, Env, QuitTo};

use super::super::step_budget;
use super::Frontend;
use super::flow_loading::LoadingScreen;
use super::flow_movie;

impl Frontend {
    /// Carries out what the screens asked for.
    pub(in crate::frontend) fn apply(&mut self, commands: Vec<Command>, host: &mut Host, env: &mut Env) {
        for command in commands {
            let (name, started) = (step_budget::Head::of(&command), std::time::Instant::now());
            self.apply_one(command, host, env);
            if let Some(warning) = step_budget::command_warning(&name, started.elapsed()) {
                log::warn!("{warning}");
            }
        }
    }

    fn apply_one(&mut self, command: Command, host: &mut Host, env: &mut Env) {
        match command {
            Command::OpenGarage => self.open_garage(host, env),
            Command::OpenTuning => self.open_tuning(host, env),
            Command::StartFreeRoam => {
                self.career.end_play();
                self.origin = QuitTo::Main;
                self.free_roam_world = true;
                self.start_free_roam(LoadingScreen::Plain, host, env);
            }
            Command::StartChallenge { race, transmission } => self.start_challenge(race, transmission, host, env),
            Command::PlayTutorial { movie, back, args } => {
                self.play_movie(&movie, flow_movie::AfterMovie::Open(back, args), host)
            }
            Command::PlayMovie { movie, then } => self.play_movie(&movie, then, host),
            Command::CustomDefaults => crate::frontend::custom_race::state::with(|state| state.reset_settings()),
            Command::StartCustomRace(start) => self.start_custom_race(*start, host, env),
            Command::SplitFirstCar(transmission) => self.split_first_car(transmission, host, env),
            Command::StartSplitRace { race, options, setup } => self.start_split_race(race, options, setup, host, env),
            Command::CoopSecondCar { handle, transmission } => self.coop_second_car(handle, transmission, host, env),
            Command::StartQuickPlay { race, car } => self.start_quick_play(race, car, host, env),
            Command::ResumeCareer => self.resume_career(host, env),
            Command::LeaveSafeHouse => self.leave_prelude(host, env),
            Command::JumpToSafeHouse => self.jump_to_safe_house(host, env),
            Command::ChallengeRival { bin } => self.challenge_rival(bin, host, env),
            Command::LaunchEvent { event } => self.launch_event(event, host, env),
            Command::JumpTo(point) => self.jump_to(point, host, env),
            Command::BlacklistStep(stepper) => self.blacklist_step(stepper, host, env),
            Command::StartFirstDrive => self.start_first_drive(host, env),
            Command::LeaveCarLot => self.leave_car_lot(host, env),
            Command::DeleteSms { id, then } => self.delete_sms(id, *then, host, env),
            Command::RestartRace => self.restart_race(host),
            Command::Resume if self.career.is_prelude() => self.start_from_safe_house(host, env),
            Command::Resume => self.resume_driving(env),
            Command::EndCoop => self.end_coop_from_pause(host, env),
            Command::SetWaypoint(target) => {
                host.scene.set_waypoint(Some(target));
                self.resume_driving(env);
            }
            Command::QuitTo(to) => {
                self.save_settings(env.changed);
                self.quit_to(to, host, env);
            }
            Command::QuitGame => {
                self.save_settings(env.changed);
                self.quit = true;
            }
            Command::NextBootStep => self.next_boot(host, env),
            Command::CareerSave => self.career_save(host, env),
            Command::CareerLoad(alias) => self.career_load(&alias, host, env),
            Command::CareerNew(alias) => self.career_new(&alias, host, env),
            Command::CareerDelete(alias) => self.career_delete(&alias, host, env),
            Command::CarLot(action) => self.car_lot_action(action, host, env),
            Command::CareerCar(action) => self.career_car_action(action, host, env),
            Command::SaveSettings => self.save_settings(env.changed),
            Command::OpenGraphics => self.graphics_requested = true,
            Command::Remap(control, slot) => crate::frontend::controller_remap::begin(self, control, slot, false),
            Command::RemapController(control, slot) => {
                crate::frontend::controller_remap::begin(self, control, slot, true)
            }
            Command::JukeboxPreview(song) => crate::audio::eatrax_preview::request(song),
            Command::JukeboxChanged(jukebox) => crate::audio::eatrax_preview::changed(jukebox),
            Command::AliasCreate(alias) => self.alias_create(&alias, host, env),
            Command::PayBust { marker } => self.pay_bust(marker, host, env),
            Command::RaceAnswer(answer) => self.race_answer(answer, host, env),
            Command::ShowResults(report) => self.show_results(report, host, env),
            Command::ShowPhoto(report) => self.show_photo(report, host, env),
            Command::ShowPursuitResults(report) => self.show_pursuit_results(report, host, env),
            Command::Switch(..)
            | Command::Push(..)
            | Command::Pop
            | Command::Confirm(_)
            | Command::Alert(_)
            | Command::EnterName(_) => {}
        }
    }
}
