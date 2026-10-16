//! The entry prompts of the world: driving into an auto shop, a safe house or a race start offers its prompt on the HUD,
//! and the Interact button enters it. The destinations are not built yet, so entering is a stub: it logs the entry and
//! flashes a placeholder card. Spec: `docs/specs/world-markers.md`.
//!
//! A mod's interior (`docs/specs/mod-world-content.md`, section 3) is entered the same way. Its entry runs the steps of
//! `modding::content::interior::machine`: the fade, the wait for its pack, the teleport, and the traffic and clock changes,
//! which the exit zone undoes.

use glam::Vec3;

use super::entry_zones::{EntryZones, Zone, ZoneKind};
use super::road::Spawn;
use super::{View, WorldScene};
use crate::hud::{InteractPopup, Prompt};
use crate::input::{Action, ActionState};
use crate::modding::content::interior::ModInterior;
use crate::modding::content::interior::machine::{Clock, Command, Machine, Plan, Snapshot, traffic_while_inside};
use crate::viewer::{Environment, Scene};
use blackbox_modding::content::{Exit, Suspend, TimeOverride};

/// How long the placeholder card stays after an enter, seconds.
const PLACEHOLDER_SECS: f32 = 2.0;

/// The entry state of the world: where the zones are and what the card shows.
#[derive(Debug, Default)]
pub(super) struct Entry {
    pub zones: EntryZones,
    pub popup: InteractPopup,
    /// The steps of the mod interior the player is going into, or is in.
    pub interior: Machine,
    /// The index in `zones.interiors` of the interior the machine runs for.
    pub current: Option<usize>,
}

impl WorldScene {
    /// One frame of the entry prompts: the card follows the zone under the car (only while driving in the chase view),
    /// and Interact enters the zone when its card is up. Interact is read only while the card is up, so a press on
    /// another control (the pad's South also handbrakes) does nothing here unless a prompt is on screen.
    pub(super) fn update_entry(&mut self, input: &ActionState, dt: f32) {
        let ready = self.entry.interior.loading_tile().is_some_and(|tile| self.residency.pack_resident(tile));
        let commands = self.entry.interior.tick(dt, ready);
        self.run_interior(commands);
        if self.entry.interior.is_inside() {
            return self.update_inside(input, dt);
        }
        if self.entry.interior.is_outside() {
            return self.update_outside(input, dt);
        }
        self.hold_card(dt);
    }

    /// Nothing offers a card while the player goes into or out of a mod interior.
    fn hold_card(&mut self, dt: f32) {
        self.entry.popup.set_target(None);
        self.entry.popup.update(dt);
    }

    /// Outside: a mod interior's entry zone first, then the stock zones.
    fn update_outside(&mut self, input: &ActionState, dt: f32) {
        let interior = self.entry_point().and_then(|point| self.entry.zones.entered_interior(point));
        if let Some(index) = interior {
            let prompt = interior_prompt(&self.entry.zones.interiors[index]);
            if enter_wanted(input, &self.entry.popup, true) {
                self.begin_interior(index);
                return;
            }
            self.entry.popup.set_target(Some(prompt));
            self.entry.popup.update(dt);
            return;
        }
        let zone = self.entered_zone().map(|zone| (zone.kind, zone.name.clone()));
        let prompt = zone.as_ref().map(|(kind, _)| prompt_for(*kind));
        if let (true, Some((kind, name))) = (enter_wanted(input, &self.entry.popup, zone.is_some()), zone.as_ref()) {
            enter_stub(&mut self.entry.popup, *kind, name);
        }
        self.entry.popup.set_target(prompt);
        self.entry.popup.update(dt);
    }

    /// Inside a mod interior: its exit zone offers the exit card, and Interact leaves.
    fn update_inside(&mut self, input: &ActionState, dt: f32) {
        let Some(index) = self.entry.current else {
            self.hold_card(dt);
            return;
        };
        let (exit, label) = {
            let interior = &self.entry.zones.interiors[index];
            (interior.exit.clone(), interior.label.clone())
        };
        let at_exit = self.car_point().is_some_and(|point| exit.shape.contains(point.to_array()));
        if !at_exit {
            self.hold_card(dt);
            return;
        }
        if enter_wanted(input, &self.entry.popup, true) {
            let commands = self.entry.interior.leave();
            self.run_interior(commands);
            return;
        }
        self.entry.popup.set_target(Some(exit_prompt(&exit, &label)));
        self.entry.popup.update(dt);
    }

    /// The entry zone the car is in, while the car drives in the chase view.
    fn entered_zone(&self) -> Option<&Zone> {
        if self.view != View::Chase {
            return None;
        }
        let drive = self.drive.as_ref()?;
        self.entry.zones.entered(drive.position())
    }

    /// The car's place, while it drives in the chase view: the only place an entry zone counts.
    fn entry_point(&self) -> Option<Vec3> {
        if self.view != View::Chase {
            return None;
        }
        self.car_point()
    }

    /// The car's place, if there is a car.
    fn car_point(&self) -> Option<Vec3> {
        self.drive.as_ref().map(|drive| drive.position())
    }

    /// Start going into a mod interior: the car's place and the settings are saved first, and the machine runs the steps.
    fn begin_interior(&mut self, index: usize) {
        let Some(drive) = self.drive.as_ref() else { return };
        let saved = Snapshot {
            pos: drive.position().to_array(),
            heading: drive.heading(),
            traffic: self.traffic_setting,
            clock: self.clock(),
        };
        let interior = self.entry.zones.interiors[index].clone();
        let plan = Plan { tile: interior.tile, spawn: interior.spawn, time: interior.time, suspend: interior.suspend };
        log::info!("entering interior {} of mod {}", interior.key, interior.owner);
        self.entry.current = Some(index);
        let commands = self.entry.interior.begin(plan, saved);
        self.run_interior(commands);
    }

    /// Carry out the machine's steps, in order. Once the machine is outside again, the interior is no longer current.
    fn run_interior(&mut self, commands: Vec<Command>) {
        for command in commands {
            match command {
                // No fade overlay exists yet: the steps keep their order and timing without it.
                Command::FadeOut | Command::FadeIn => log::debug!("interior: fade (no overlay yet)"),
                Command::Pin { tile, on } => self.residency.pin(tile, on),
                Command::Teleport { pos, heading } => self.place_car_or_leave(pos, heading),
                Command::Suspend(suspend) => self.suspend_for_interior(suspend),
                Command::SetClock(time) => self.set_interior_clock(time),
                Command::Restore(saved) => self.restore_from(saved),
                Command::Notice(line) => {
                    self.entry
                        .popup
                        .flash(Prompt { title: "Interior".to_string(), line: line.to_string() }, PLACEHOLDER_SECS);
                }
            }
        }
        if self.entry.interior.is_outside() {
            self.entry.current = None;
        }
    }

    /// Put the car at the interior's spawn. Without ground there, the entry is undone at once: the machine leaves, and the
    /// car goes back to where it stood.
    fn place_car_or_leave(&mut self, pos: [f32; 3], heading: f32) {
        if self.place_car_at(pos, heading) {
            return;
        }
        log::warn!("interior: no ground at ({:.0}, {:.0}, {:.0}); leaving again", pos[0], pos[1], pos[2]);
        let commands = self.entry.interior.leave();
        self.run_interior(commands);
    }

    /// Put the car exactly at a place, standing still. An interior has no road, so the road search is not used.
    fn place_car_at(&mut self, pos: [f32; 3], heading: f32) -> bool {
        let Some(drive) = self.drive.as_mut() else { return false };
        let spawn = Spawn { position: Vec3::from_array(pos), heading };
        let collision = self.residency.collision();
        drive.spawn(spawn, collision, &self.physics.surfaces)
    }

    /// Stop what the interior names. Cops leave with the chase, so cops or pursuit stop the chase.
    fn suspend_for_interior(&mut self, suspend: Suspend) {
        self.apply_traffic_setting(traffic_while_inside(self.traffic_setting, suspend));
        let ends_chase = suspend.pursuit || suspend.cops;
        if !ends_chase {
            return;
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.stop_pursuit();
        }
    }

    /// The hour and whether the clock is held, as the interior sets them. The rate is kept.
    fn set_interior_clock(&mut self, time: TimeOverride) {
        let rate = self.environment().map_or(60.0, |env| env.seconds_per_hour);
        self.set_environment(Environment { hour: time.hour, seconds_per_hour: rate });
        self.set_time_held(time.freeze);
    }

    /// Give back the traffic settings and the clock saved at entry.
    fn restore_from(&mut self, saved: Snapshot) {
        self.apply_traffic_setting(saved.traffic);
        self.set_environment(saved.clock.env);
        self.set_time_held(saved.clock.held);
    }

    /// The traffic settings and the clock now.
    fn clock(&self) -> Clock {
        let env = self.environment().unwrap_or(Environment { hour: 0.0, seconds_per_hour: 60.0 });
        Clock { env, held: self.time_held() }
    }
}

/// The entry card of a mod interior: its label, and its line (the manifest's, or the game's).
fn interior_prompt(interior: &ModInterior) -> Prompt {
    let line = interior.prompt.clone().unwrap_or_else(|| "Press Enter to go in".to_string());
    Prompt { title: interior.label.clone(), line }
}

/// The exit card of a mod interior.
fn exit_prompt(exit: &Exit, label: &str) -> Prompt {
    let line = exit.prompt.clone().unwrap_or_else(|| "Press Enter to leave".to_string());
    Prompt { title: label.to_string(), line }
}

/// Whether this frame's Interact press enters the zone: the car is in one, the card is up and not holding a placeholder,
/// and Interact went down this frame. Nothing else reads the press as an entry.
fn enter_wanted(input: &ActionState, popup: &InteractPopup, in_zone: bool) -> bool {
    in_zone && popup.is_up() && !popup.is_flashing() && input.just_pressed(Action::Interact)
}

/// The card's text for a zone.
fn prompt_for(kind: ZoneKind) -> Prompt {
    Prompt { title: capitalise(kind.label()), line: "Press Enter to go in".to_string() }
}

/// The stub destination: log the entry and flash the placeholder. Replace with the real entry (a race, the safe house
/// or the auto shop) when those exist.
fn enter_stub(popup: &mut InteractPopup, kind: ZoneKind, name: &str) {
    log::info!("entering {} {name} (stub: the destination is not built yet)", kind.label());
    let placeholder = Prompt { title: format!("Entering {}", kind.label()), line: "Not built yet".to_string() };
    popup.flash(placeholder, PLACEHOLDER_SECS);
}

/// Upper-case the first letter of a label.
fn capitalise(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::keyboard::KeyCode;

    use super::*;

    #[test]
    fn the_card_names_the_place_with_a_capital() {
        assert_eq!(prompt_for(ZoneKind::AutoShop).title, "Auto shop");
        assert_eq!(prompt_for(ZoneKind::SafeHouse).title, "Safe house");
        assert_eq!(prompt_for(ZoneKind::Race).title, "Race");
        assert_eq!(capitalise(""), "");
    }

    /// Input with Interact pressed this frame (`pressed`) or not, after a frame with nothing down.
    fn input(pressed: bool) -> ActionState {
        let bindings = crate::input::Bindings::default();
        let mut input = ActionState::default();
        input.update_for_test(&bindings, &[], &[], false);
        let keys: &[KeyCode] = if pressed { &[KeyCode::Enter] } else { &[] };
        input.update_for_test(&bindings, keys, &[], false);
        input
    }

    /// A popup whose card is up with a zone's prompt.
    fn up() -> InteractPopup {
        let mut popup = InteractPopup::default();
        popup.set_target(Some(prompt_for(ZoneKind::AutoShop)));
        popup.update(1.0);
        popup
    }

    #[test]
    fn interact_enters_only_from_a_zone_with_its_card_up() {
        let pressed = input(true);
        assert!(enter_wanted(&pressed, &up(), true), "in a zone, card up, pressed");
        assert!(!enter_wanted(&pressed, &up(), false), "no zone, no entry");
        assert!(!enter_wanted(&input(false), &up(), true), "not pressed, no entry");
        assert!(!enter_wanted(&pressed, &InteractPopup::default(), true), "the card is not up yet");
        let mut flashing = up();
        flashing.flash(prompt_for(ZoneKind::Race), 1.0);
        assert!(!enter_wanted(&pressed, &flashing, true), "a placeholder holds the card");
    }

    #[test]
    fn the_stub_flashes_a_placeholder_for_the_entry() {
        let mut popup = InteractPopup::default();
        popup.set_target(Some(prompt_for(ZoneKind::SafeHouse)));
        popup.update(1.0);
        assert!(popup.is_up());
        enter_stub(&mut popup, ZoneKind::SafeHouse, "menu_gate_safehouse/zone_safehouse_1");
        assert!(popup.is_flashing(), "the placeholder holds the card for its time");
        popup.update(1.0);
        assert_eq!(popup.drawn().map(|p| p.title.as_str()), Some("Entering safe house"));
    }
}
