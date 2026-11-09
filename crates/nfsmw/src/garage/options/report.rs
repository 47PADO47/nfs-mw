//! The options as text, for the console.

use super::{Choice, Slot, VehicleOptions};

fn choice_text(at: usize, slot: &Slot, choice: &Choice) -> String {
    let mark = match at == slot.current {
        true => "*",
        false => "",
    };
    let price = choice.price.map(|price| format!(" ${price}")).unwrap_or_default();
    let blocked = choice.blocked.as_ref().map(|reason| format!(" ({reason})")).unwrap_or_default();
    format!("{at}: {}{price}{blocked}{mark}", choice.label)
}

impl VehicleOptions {
    /// Every slot with its numbered choices; the installed one is starred.
    pub(crate) fn slots_text(&self) -> String {
        let kind = match self.bike {
            true => "bike",
            false => "car",
        };
        let mut lines = vec![format!("{} ({kind}, {}); `vehicle set <slot> <number|id|name>`", self.name, self.folder)];
        let mut group = "";
        for slot in &self.slots {
            if slot.group != group {
                group = &slot.group;
                lines.push(format!("{group}:"));
            }
            let choices: Vec<String> =
                slot.choices.iter().enumerate().map(|(at, choice)| choice_text(at, slot, choice)).collect();
            lines.push(format!("  {} ({}): {}", slot.id, slot.label, choices.join(", ")));
        }
        if let Some(paint) = self.paintwork {
            let own = paint.original.map(|[r, g, b]| format!(" (own #{r:02X}{g:02X}{b:02X})")).unwrap_or_default();
            let shown = match paint.custom {
                Some([r, g, b]) => format!("#{r:02X}{g:02X}{b:02X}"),
                None => "the vehicle's own".to_owned(),
            };
            lines.push(format!("body colour: {shown}{own}; `vehicle color <RRGGBB|stock>`"));
        }
        lines.join("\n")
    }

    /// The groups with their slots and state, and what the vehicle's data lacks.
    pub(crate) fn groups_text(&self) -> String {
        let mut lines = Vec::new();
        for group in self.groups() {
            let state = match (group.switchable, group.on) {
                (true, true) => ", on".to_owned(),
                (true, false) => ", off".to_owned(),
                (false, _) => String::new(),
            };
            let kind = match group.switchable {
                true => "switchable",
                false => "not switchable",
            };
            lines.push(format!(
                "{}: {} ({} changed, {kind}{state})",
                group.name,
                group.slots.join(", "),
                group.changed
            ));
        }
        lines.push("`vehicle group <name> on|off|stock`".to_owned());
        lines.extend(self.missing.iter().map(|line| format!("missing: {line}")));
        lines.join("\n")
    }
}
