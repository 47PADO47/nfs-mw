//! A bike manifest's `[bike.dash]` as the game's live screen of the cluster (`nfsmw_data::car::mods::bike_dash`).

use blackbox_modding::content::{BikeDash as Table, BikeDashAlign, BikeDashChannel};
use nfsmw_data::car::mods::bike_dash::{self, Align, Channel, Colors, Dash, Field};

/// Register the screen `table` describes for the bike in `folder`.
pub(super) fn publish(folder: &str, table: &Table) {
    let own = table.colors;
    let colors = Colors {
        background: own.background,
        lit: own.lit,
        dim: own.dim,
        text: own.text,
        label: own.label,
        gear: own.gear,
    };
    let fields = table
        .field
        .iter()
        .map(|field| Field {
            channel: match field.channel {
                BikeDashChannel::Speed => Channel::Speed,
                BikeDashChannel::Gear => Channel::Gear,
                BikeDashChannel::RpmBar => Channel::RpmBar,
            },
            rect: field.rect,
            color: field.color,
            background: field.background,
            align: match field.align {
                BikeDashAlign::Left => Align::Left,
                BikeDashAlign::Center => Align::Center,
                BikeDashAlign::Right => Align::Right,
            },
            segments: field.segments,
        })
        .collect();
    bike_dash::publish(folder, Dash::new(&table.texture, table.window, colors, fields));
}
