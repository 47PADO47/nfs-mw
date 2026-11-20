//! The rider settings as the config file holds them: a riding style per class of bike by name, the fingers as numbers.

use toml::Value;

use super::{BikeClass, Partial};

/// The `(key, value)` pairs of the rider settings `changes` carries, for the config file.
pub(super) fn entries(changes: &Partial) -> Vec<(&'static str, Value)> {
    let styles = BikeClass::ALL.into_iter().filter_map(|class| {
        changes.riding_style_of(class).map(|style| (class.key(), Value::String(style.to_string())))
    });
    let brake = changes.brake_fingers.map(|v| ("brake_fingers", Value::Integer(i64::from(v.count()))));
    let clutch = changes.clutch_fingers.map(|v| ("clutch_fingers", Value::Integer(i64::from(v.count()))));
    styles.chain(brake).chain(clutch).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{BrakeFingers, ClutchFingers, RidingStyle};

    #[test]
    fn only_what_changed_is_written_styles_by_name_and_fingers_as_numbers() {
        assert!(entries(&Partial::default()).is_empty());
        let changes = Partial {
            riding_style_motard: Some(RidingStyle::OldSchool),
            brake_fingers: Some(BrakeFingers::One),
            clutch_fingers: Some(ClutchFingers::Four),
            ..Partial::default()
        };
        assert_eq!(
            entries(&changes),
            vec![
                ("riding_style_motard", Value::String("old-school".into())),
                ("brake_fingers", Value::Integer(1)),
                ("clutch_fingers", Value::Integer(4)),
            ]
        );
    }
}
