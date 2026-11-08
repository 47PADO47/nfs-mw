//! A bike manifest's `[bike.dash]` as the game's live screen of the cluster (`nfsmw_data::car::mods::bike_dash`).

use blackbox_modding::content::BikeDash as Table;
use nfsmw_data::car::mods::bike_dash::{self, Colors, Dash};

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
    bike_dash::publish(folder, Dash::new(&table.texture, table.window, colors));
}
