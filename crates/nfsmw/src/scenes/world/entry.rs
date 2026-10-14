//! The entry prompts of the world: driving into an auto shop, a safe house or a race start offers its prompt on the HUD,
//! and the Interact button enters it. The destinations are not built yet, so entering is a stub: it logs the entry and
//! flashes a placeholder card. Spec: `docs/specs/world-markers.md`.

use super::entry_zones::{EntryZones, Zone, ZoneKind};
use super::{View, WorldScene};
use crate::hud::{InteractPopup, Prompt};
use crate::input::{Action, ActionState};

/// How long the placeholder card stays after an enter, seconds.
const PLACEHOLDER_SECS: f32 = 2.0;

/// The entry state of the world: where the zones are and what the card shows.
#[derive(Debug, Default)]
pub(super) struct Entry {
    pub zones: EntryZones,
    pub popup: InteractPopup,
}

impl WorldScene {
    /// One frame of the entry prompts: the card follows the zone under the car (only while driving in the chase view),
    /// and Interact enters the zone when its card is up. Interact is read only while the card is up, so a press on
    /// another control (the pad's South also handbrakes) does nothing here unless a prompt is on screen.
    pub(super) fn update_entry(&mut self, input: &ActionState, dt: f32) {
        let zone = self.entered_zone().map(|zone| (zone.kind, zone.name.clone()));
        let prompt = zone.as_ref().map(|(kind, _)| prompt_for(*kind));
        if let (true, Some((kind, name))) = (enter_wanted(input, &self.entry.popup, zone.is_some()), zone.as_ref()) {
            enter_stub(&mut self.entry.popup, *kind, name);
        }
        self.entry.popup.set_target(prompt);
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
