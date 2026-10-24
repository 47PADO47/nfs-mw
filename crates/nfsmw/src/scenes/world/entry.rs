//! The entry prompts of the world: driving into an auto shop, a safe house or a race start offers its prompt on the HUD,
//! and the Interact button enters it. Auto shops request the normal garage; other destinations follow the career
//! entry flow or display their placeholder card. Spec: `docs/specs/world-markers.md`.
//!
//! A mod's interior (`docs/specs/mod-world-content.md`, section 3) is entered the same way. Its entry runs the steps of
//! `modding::content::interior::machine`: the fade, the wait for its pack, the teleport, and the traffic and clock changes,
//! which the exit zone undoes.

use glam::Vec3;

use super::career::RaceEntry;
use super::entry_zones::{EntryZones, ZoneKind};
use super::players::{LEAD, PlayerId};
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

/// How far behind the lead a co-op guest's car is put when the lead is placed, metres.
const GUEST_GAP: f32 = 12.0;

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
        // A race holds every other entry: the card, the zones and the interiors wait for the result.
        if self.race.is_some() {
            return self.hold_card(dt);
        }
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

    /// A race zone was entered: the race starts. An open career race counts in the career; a race outside the career
    /// (or one the Blacklist does not list) is a free run. A race that cannot start flashes why on the card.
    fn enter_race_zone(&mut self, name: &str, career: bool) {
        let options = super::race::RaceOptions { career, ..Default::default() };
        match self.start_race(name, options) {
            Ok(line) => log::info!("race: {line}"),
            Err(why) => {
                log::warn!("race {name}: {why}");
                let card = Prompt { title: "Race".to_string(), line: why };
                self.entry.popup.flash(card, PLACEHOLDER_SECS);
            }
        }
    }

    /// Nothing offers a card while the player goes into or out of a mod interior.
    fn hold_card(&mut self, dt: f32) {
        self.entry.popup.set_target(None);
        self.entry.popup.update(dt);
    }

    /// Outside: the first player whose car is in an entry zone offers its card, and Interact on that player's own
    /// controls enters it. The lead is tested first; a mod interior's zone counts before the stock zones.
    fn update_outside(&mut self, input: &ActionState, dt: f32) {
        let points = self.entry_points();
        let Some((id, hit)) = first_holder(&points, |point| zone_hit(&self.entry.zones, point)) else {
            self.entry.popup.set_target(None);
            self.entry.popup.update(dt);
            return;
        };
        let pressed = self.actions_of(id, input).is_some_and(|actions| enter_wanted(actions, &self.entry.popup, true));
        match hit {
            Hit::Interior(index) => self.update_interior_zone(index, pressed, dt),
            Hit::Zone { kind, name } => self.update_stock_zone(kind, &name, pressed, dt),
        }
    }

    /// A mod interior's entry zone holds the car: Interact goes into it, otherwise its card is offered.
    fn update_interior_zone(&mut self, index: usize, pressed: bool, dt: f32) {
        if pressed {
            self.begin_interior(index);
            return;
        }
        let prompt = interior_prompt(&self.entry.zones.interiors[index]);
        self.entry.popup.set_target(Some(prompt));
        self.entry.popup.update(dt);
    }

    /// A stock zone holds the car: Interact enters it (a race, or the career's entry, or the stub), and its card is
    /// offered either way.
    fn update_stock_zone(&mut self, kind: ZoneKind, name: &str, pressed: bool, dt: f32) {
        if kind == ZoneKind::AutoShop && !self.garage_entry_allowed() {
            return self.hold_card(dt);
        }
        if pressed {
            self.enter_stock_zone(kind, name);
        }
        self.entry.popup.set_target(Some(prompt_for(kind)));
        self.entry.popup.update(dt);
    }

    /// Interact in a stock zone: a locked race flashes its card; a race starts; any other zone goes to the career, or to
    /// the stub when the career does not take it.
    fn enter_stock_zone(&mut self, kind: ZoneKind, name: &str) {
        let race = match kind {
            ZoneKind::Race => self.career_enter_race(name),
            ZoneKind::AutoShop | ZoneKind::SafeHouse | ZoneKind::CarLot => RaceEntry::NotCareer,
        };
        if race == RaceEntry::Locked {
            flash_locked(&mut self.entry.popup, name);
            return;
        }
        if kind == ZoneKind::Race {
            let career = race == RaceEntry::Open || self.career.handle.is_active();
            self.enter_race_zone(name, career);
            return;
        }
        if kind == ZoneKind::AutoShop {
            if !self.garage_entry_allowed() {
                return;
            }
            self.remember_garage_exit(name);
            if self.career.handle.is_active() {
                self.career_visit_zone(kind, name);
            }
            self.garage.open_requested = true;
            return;
        }
        if self.career_enter_zone(kind, name) {
            return;
        }
        enter_stub(&mut self.entry.popup, kind, name);
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

    /// Every player's car place, the lead first, while the car drives in the chase view: the only places an entry zone
    /// counts. Empty otherwise.
    fn entry_points(&self) -> Vec<(PlayerId, Vec3)> {
        if self.view != View::Chase {
            return Vec::new();
        }
        self.players.iter().map(|player| (player.id, player.drive.position())).collect()
    }

    /// The actions of player `id` this frame. The lead's are the frame's input; a guest's are its own seat's, and there
    /// are none (idle) when it has no seat.
    fn actions_of<'a>(&'a self, id: PlayerId, input: &'a ActionState) -> Option<&'a ActionState> {
        if id == LEAD {
            return Some(input);
        }
        self.inputs.get(id)
    }

    /// The car's place, if there is a car.
    fn car_point(&self) -> Option<Vec3> {
        self.players.lead().map(|drive| drive.position())
    }

    /// Start going into a mod interior: the car's place and the settings are saved first, and the machine runs the steps.
    fn begin_interior(&mut self, index: usize) {
        let Some(drive) = self.players.lead() else { return };
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
        self.entry.interior.set_key(&interior.key);
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

    /// Put the lead's car exactly at a place, standing still, and each co-op guest's car just behind it, facing the same
    /// way. An interior has no road, so the road search is not used for the lead.
    fn place_car_at(&mut self, pos: [f32; 3], heading: f32) -> bool {
        let Some(drive) = self.players.lead_mut() else { return false };
        let spawn = Spawn { position: Vec3::from_array(pos), heading };
        let collision = self.residency.collision();
        if !drive.spawn(spawn, collision, &self.physics.surfaces) {
            return false;
        }
        let near = behind_point(pos, heading, GUEST_GAP);
        for guest in self.players.guests_mut() {
            guest.drive.respawn_near(near, Some(heading));
        }
        true
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
        self.set_environment(Environment {
            hour: time.hour,
            seconds_per_hour: rate,
            ..self.environment().unwrap_or_default()
        });
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
        let env = self.environment().unwrap_or(Environment { hour: 0.0, seconds_per_hour: 60.0, ..Default::default() });
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

/// What the entry zones hold at a point: a mod interior (its index in `zones.interiors`), or a stock zone.
#[derive(Debug, Clone, PartialEq)]
enum Hit {
    Interior(usize),
    Zone { kind: ZoneKind, name: String },
}

/// What the entry zones hold at `point`: a mod interior's zone first, then the stock zones.
fn zone_hit(zones: &EntryZones, point: Vec3) -> Option<Hit> {
    if let Some(index) = zones.entered_interior(point) {
        return Some(Hit::Interior(index));
    }
    let zone = zones.entered(point)?;
    Some(Hit::Zone { kind: zone.kind, name: zone.name.clone() })
}

/// The first player, in `points` order, whose point the test holds, with what the test gave. `points` lists the lead
/// first, so a tie goes to the lead.
fn first_holder<T>(points: &[(PlayerId, Vec3)], holds: impl Fn(Vec3) -> Option<T>) -> Option<(PlayerId, T)> {
    points.iter().find_map(|&(id, point)| holds(point).map(|hit| (id, hit)))
}

/// The ground point `distance` metres behind `pos` along `heading` (forward is `(cos h, sin h)`), where a co-op guest's
/// car is put.
fn behind_point(pos: [f32; 3], heading: f32, distance: f32) -> [f32; 2] {
    [pos[0] - heading.cos() * distance, pos[1] - heading.sin() * distance]
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

/// The card for a race the career has not opened yet. The Blacklist has not reached its bin, or its rival is still
/// locked (`career-blacklist.md` section 2). The original's label is not in the spec; this one is a stand-in.
fn flash_locked(popup: &mut InteractPopup, name: &str) {
    log::info!("career: race {name} is locked");
    let card = Prompt { title: "Race locked".to_string(), line: "Not open yet".to_string() };
    popup.flash(card, PLACEHOLDER_SECS);
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

    /// A zone test: a 6 m sphere at the origin.
    fn in_sphere(point: Vec3) -> Option<()> {
        crate::scenes::world::entry_zones::sphere_contains(Vec3::ZERO, 6.0, point).then_some(())
    }

    #[test]
    fn lead_wins_a_tie() {
        let inside = Vec3::new(1.0, 0.0, 0.0);
        let points = [(LEAD, inside), (PlayerId(1), inside)];
        assert_eq!(first_holder(&points, in_sphere), Some((LEAD, ())));
    }

    #[test]
    fn guest_in_zone_triggers() {
        let points = [(LEAD, Vec3::new(50.0, 0.0, 0.0)), (PlayerId(1), Vec3::ZERO)];
        assert_eq!(first_holder(&points, in_sphere), Some((PlayerId(1), ())));
    }

    #[test]
    fn nobody_in_zone() {
        let points = [(LEAD, Vec3::new(50.0, 0.0, 0.0)), (PlayerId(1), Vec3::new(0.0, 50.0, 0.0))];
        assert_eq!(first_holder(&points, in_sphere), None);
        assert_eq!(first_holder(&[], in_sphere), None);
    }

    #[test]
    fn a_guest_is_put_behind_the_lead() {
        assert_eq!(behind_point([100.0, 50.0, 0.0], 0.0, GUEST_GAP), [88.0, 50.0]);
    }
}
