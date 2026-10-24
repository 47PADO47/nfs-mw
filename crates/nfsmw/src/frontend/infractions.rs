//! The infractions screen (`Infractions.fng`): the ticket the player gets after a bust in free roam, with the fines due
//! and the two ways to pay (`docs/specs/busted-sequence.md` sections 6.3 and 7). The flow opens it with the chase's
//! [`Ticket`] (`Screens::open_with`). A payment sends `Command::PayBust`; the bust takes it and the game goes back to
//! free roam. In career mode the ticket also shows the car's unserved infractions and fines; the cash is held on the
//! ticket but its text object is not known yet, so it is not drawn. The marker count is the career's Get Out of Jail
//! markers.

use blackbox_feng::ids::{PAD_LEFT, PAD_RIGHT};
use blackbox_pursuit::Infraction;
use blackbox_text::label_hash;
use nfsmw_career::economy::fine_of_counters;
use nfsmw_career::economy::markers::MarkerKind;

use super::disabled;
use super::ids::INIT_COMPLETE;
use super::logic::{Command, Cx, ScreenLogic};
use crate::career::Career;
use crate::i18n::{self, fmt::grouped};

/// Objects of `Infractions.fng` by name hash (its dump, `nfsmw dump-screen`). The ticket's values: this chase's infraction
/// count and fines, the unserved count and fines (0 outside career mode), and the total due.
const VIOLATION: u32 = 0x72ec_6e9c;
const PURSUIT_COUNT: u32 = 0xb967_f64d;
const PURSUIT_FINES: u32 = 0xbd66_334a;
const UNSERVED_COUNT: u32 = 0x5344_f2a6;
const UNSERVED_FINES: u32 = 0xa4c7_9522;
const TOTAL: u32 = 0x854a_f1f4;
/// The "Pay with Marker" text, dimmed while the player has no marker.
const MARKER_TEXT: u32 = 0xf936_3f30;
/// The two payment buttons, by GUID: the holders take the focus and send the press. Cash is on the right, Marker on the
/// left (the package's left and right moves).
const CASH_HOLDER: u32 = 0x000f_72c2;
const MARKER_HOLDER: u32 = 0x000f_70e8;
// The package posts nothing to the game when a payment button is pressed: its two game responses only forward the
// pad's left and right moves (`PAD_LEFT`, `PAD_RIGHT`). The press is the button's own `BUTTON_PRESSED` (the original
// handler tells the buttons apart by their name hashes `0xb8a7c6cc` for Marker and `0xb8a7c6cd` for Cash, which belong
// to the holders above), so the logic listens for it through `ScreenLogic::pressed`.

/// The language labels of the violation line's header (`MW_VIOLATION_HEADER`) and of no offence (`COMMON_NONE`).
const VIOLATION_HEADER: u32 = 0xD3B0_5B5F;
const NONE: u32 = 0x8CDC_3937;

/// The chase's bill: each infraction with its fine (in HUD order) and the markers the player has. In career mode it also
/// carries the car's unserved infractions and their fines, and the cash; outside career mode those read 0 and `None`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ticket {
    pub lines: Vec<(Infraction, u32)>,
    /// Markers in stock. The marker inventory is not kept yet, so the career passes 0 (`career-economy-flows.md` 6.5).
    pub markers: u32,
    /// The car's unserved infractions, counted (every kind's count, not the kinds).
    pub unserved_count: u32,
    /// The fine of the car's unserved infractions, saturating.
    pub unserved_fines: u32,
    /// The cash in hand; `None` outside career mode.
    pub cash: Option<u32>,
}

impl Ticket {
    /// The ticket of a bust in career mode: the chase's lines, the current car's unserved infractions and fines, and the
    /// cash. A car with no career record owes nothing.
    pub fn with_career(lines: &[(Infraction, u32)], career: &Career) -> Ticket {
        let settings = &career.settings;
        let (unserved_count, unserved_fines) = match career.stable.career_for_handle(settings.car_handle) {
            Some(record) => {
                let fines = u32::try_from(fine_of_counters(&record.unserved)).unwrap_or(u32::MAX);
                (record.unserved_total(), fines)
            }
            None => (0, 0),
        };
        Ticket {
            lines: lines.to_vec(),
            markers: career.markers.count(MarkerKind::JailFree),
            unserved_count,
            unserved_fines,
            cash: Some(settings.cash),
        }
    }

    /// The fines of every line, saturating.
    pub fn fines(&self) -> u32 {
        self.lines.iter().fold(0u32, |sum, (_, fine)| sum.saturating_add(*fine))
    }

    /// The total on the bill: in career mode the unserved fines (what cash pays, the chase's included once recorded),
    /// otherwise the chase's fines.
    pub fn total_due(&self) -> u32 {
        match self.cash {
            Some(_) => self.unserved_fines,
            None => self.fines(),
        }
    }

    /// The line on the ticket that names the offences after the "Violation:" header, or says none, in the language's
    /// own words.
    fn violation_text(&self) -> String {
        let header = i18n::game_or_error(VIOLATION_HEADER);
        let names: Vec<String> = self.lines.iter().map(|(offence, _)| i18n::game_or_error(label(*offence))).collect();
        violation_line(&header, &names, &i18n::game_or_error(NONE))
    }
}

/// The header followed by the offences, comma separated, or by `none` when there is no offence.
fn violation_line(header: &str, names: &[String], none: &str) -> String {
    if names.is_empty() {
        return format!("{header} {none}");
    }
    format!("{header} {}", names.join(", "))
}

/// The language label of the infraction's name, the pursuit HUD's and the busted screens' `INFRACTION_*`
/// (`docs/specs/career-text.md` section 6.3; mask `0x01` is `INFRACTION_SPEEDING`, mask `0x02` `INFRACTION_RACING`).
pub fn label(infraction: Infraction) -> u32 {
    let key = match infraction {
        Infraction::Speeding => "INFRACTION_SPEEDING",
        Infraction::Racing => "INFRACTION_RACING",
        Infraction::RecklessDriving => "INFRACTION_RECKLESS_DRIVING",
        Infraction::Assault => "INFRACTION_ASSULTING_OFFICER",
        Infraction::HitAndRun => "INFRACTION_HITNRUN",
        Infraction::DamageToProperty => "INFRACTION_PROPERTY_DAMAGE",
        Infraction::ResistingArrest => "INFRACTION_RESISTING_ARREST",
        Infraction::OffRoad => "INFRACTION_OFFROAD",
    };
    label_hash(key)
}

/// An amount as the ticket shows it, grouped by thousands ("20,000" in English).
fn amount(value: u32) -> String {
    grouped(i64::from(value))
}

/// Shows a [`Ticket`] and answers the payment buttons.
pub struct InfractionsLogic {
    ticket: Ticket,
}

impl InfractionsLogic {
    pub fn new(ticket: Ticket) -> Self {
        Self { ticket }
    }

    fn fill(&self, cx: &mut Cx) {
        let count = self.ticket.lines.len().to_string();
        let fines = amount(self.ticket.fines());
        set_text(cx, VIOLATION, &self.ticket.violation_text());
        set_text(cx, PURSUIT_COUNT, &count);
        set_text(cx, PURSUIT_FINES, &fines);
        set_text(cx, UNSERVED_COUNT, &self.ticket.unserved_count.to_string());
        set_text(cx, UNSERVED_FINES, &amount(self.ticket.unserved_fines));
        set_text(cx, TOTAL, &amount(self.ticket.total_due()));
    }

    /// Moves the focus to Marker, when the player has one.
    fn focus_marker(&self, cx: &mut Cx) {
        if self.ticket.markers == 0 {
            return;
        }
        focus(cx, MARKER_HOLDER);
    }

    /// A payment with the pressed button. A marker with none in stock does nothing.
    fn pay(&self, cx: &mut Cx, marker: bool) {
        if marker && self.ticket.markers == 0 {
            return;
        }
        cx.send(Command::PayBust { marker });
    }
}

fn set_text(cx: &mut Cx, name: u32, text: &str) {
    let Some(o) = cx.object(name) else { return };
    cx.rt.set_text(o, text);
}

/// Makes a holder the current button.
fn focus(cx: &mut Cx, guid: u32) {
    let Some(o) = cx.guid(guid) else { return };
    cx.rt.focus_button(o);
}

/// Caps the alpha of a disabled text. The package's scripts set the alpha every frame, so this runs each tick.
fn dim(cx: &mut Cx, name: u32) {
    let Some(o) = cx.object(name) else { return };
    let Some(alpha) = cx.rt.alpha(o) else { return };
    cx.rt.set_alpha(o, alpha.min(disabled::TEXT_ALPHA));
}

impl ScreenLogic for InfractionsLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
        focus(cx, CASH_HOLDER);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            INIT_COMPLETE => focus(cx, CASH_HOLDER),
            PAD_LEFT => self.focus_marker(cx),
            PAD_RIGHT => focus(cx, CASH_HOLDER),
            _ => {}
        }
    }

    fn pressed(&mut self, cx: &mut Cx, button: u32) {
        match button {
            CASH_HOLDER => self.pay(cx, false),
            MARKER_HOLDER => self.pay(cx, true),
            _ => {}
        }
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.ticket.markers > 0 {
            return;
        }
        dim(cx, MARKER_TEXT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_have_a_comma_every_three_digits() {
        assert_eq!(amount(0), "0");
        assert_eq!(amount(999), "999");
        assert_eq!(amount(1000), "1,000");
        assert_eq!(amount(80_000), "80,000");
        assert_eq!(amount(1_234_567), "1,234,567");
    }

    #[test]
    fn the_ticket_totals_its_lines_and_names_them() {
        let lines = vec![(Infraction::Speeding, 150), (Infraction::OffRoad, 75)];
        let ticket = Ticket { lines, ..Ticket::default() };
        assert_eq!(ticket.fines(), 225);
        let names = ["Fast".to_owned(), "Off the road".to_owned()];
        assert_eq!(violation_line("Ticket:", &names, "-"), "Ticket: Fast, Off the road");
    }

    #[test]
    fn career_ticket_bills_the_unserved_fines_and_the_cash() {
        let lines = [(Infraction::Speeding, 150)];
        let ticket = Ticket { unserved_count: 3, unserved_fines: 1_250, cash: Some(4_000), ..Ticket::default() };
        assert_eq!(ticket.total_due(), 1_250, "cash pays the unserved fines");
        assert_eq!(ticket.fines(), 0, "the chase has no lines here");
        let plain = Ticket { lines: lines.to_vec(), unserved_fines: 9, ..Ticket::default() };
        assert_eq!(plain.total_due(), 150, "outside career mode the bill is the chase's fines");
    }

    #[test]
    fn a_clean_ticket_names_no_offence() {
        let ticket = Ticket::default();
        assert_eq!(ticket.fines(), 0);
        assert_eq!(violation_line("Ticket:", &[], "nothing"), "Ticket: nothing");
    }

    #[test]
    fn every_infraction_has_a_name_of_its_own() {
        let labels: std::collections::HashSet<u32> = Infraction::ALL.into_iter().map(label).collect();
        assert_eq!(labels.len(), Infraction::ALL.len());
        assert_eq!(label(Infraction::Speeding), label_hash("INFRACTION_SPEEDING"));
        assert_eq!(label(Infraction::OffRoad), label_hash("INFRACTION_OFFROAD"));
    }
}
