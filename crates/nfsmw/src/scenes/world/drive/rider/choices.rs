//! What the player chose about the rider (`riding_style_road`, `riding_style_motard`, `riding_style_offroad`, `brake_fingers`,
//! `clutch_fingers`), and which of the three riding styles a bike takes.
//!
//! The host pushes the settings each frame ([`set`]); every rider reads them when it is posed.
//!
//! RIDE 6's riding style page shows its own pictures for road, motard and off-road bikes; the player picks one style per group. A
//! bike's group follows from what its `contacts.json` says it is: its animation kind (`style.name`, the graph it plays) and its input
//! handler (`docs/specs/ride6-rider-styles.md`, section 8).

use std::sync::atomic::{AtomicU8, Ordering};

use crate::settings::{BikeClass, BrakeFingers, ClutchFingers, Settings};

/// The blueprint's `RidingStyle` (0 to 2) of each class, by [`BikeClass::index`].
static RIDING_STYLES: [AtomicU8; 3] = [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)];
/// The places of the brake and clutch finger choices in `BrakeFingers::ALL` and `ClutchFingers::ALL`: two fingers on the brake (the
/// middle one) and the clutch clip's two on the clutch (the first).
static BRAKE_FINGERS: AtomicU8 = AtomicU8::new(1);
static CLUTCH_FINGERS: AtomicU8 = AtomicU8::new(0);

/// Tell the riders what the player chose.
pub fn set(settings: &Settings) {
    for class in BikeClass::ALL {
        RIDING_STYLES[class.index()].store(settings.riding_style_of(class).blueprint(), Ordering::Relaxed);
    }
    BRAKE_FINGERS.store(settings.brake_fingers.graph_index(), Ordering::Relaxed);
    CLUTCH_FINGERS.store(settings.clutch_fingers.graph_index(), Ordering::Relaxed);
    // The bike steering is a choice of the player's for bikes too, but it is the bike's, not the rider's.
    super::super::sim::set_bike_steering(settings.bike_steering);
}

/// The riding style the player chose for the bikes of `class`, as the blueprint's `RidingStyle` (0 balanced, 1 shoulders out, 2 old
/// school).
pub fn riding_style(class: BikeClass) -> u8 {
    RIDING_STYLES[class.index()].load(Ordering::Relaxed)
}

/// The fingers the player chose for the brake and the clutch lever.
pub fn fingers() -> (BrakeFingers, ClutchFingers) {
    let brake = usize::from(BRAKE_FINGERS.load(Ordering::Relaxed));
    let clutch = usize::from(CLUTCH_FINGERS.load(Ordering::Relaxed));
    (
        BrakeFingers::ALL.get(brake).copied().unwrap_or_default(),
        ClutchFingers::ALL.get(clutch).copied().unwrap_or_default(),
    )
}

/// The group of a bike by the graph it plays (`kind`: the importer's `style.name`, `race`, `naked`, `motard`, `scooter` or `enduro`)
/// and its input handler class (`Motard_InputHandler`).
///
/// - The motard group is the bikes with a supermoto's handler and those whose kind is `motard`. The Honda CRF 450 R SMT is one:
///   RIDE 6's correction table makes its type `enduro` (it plays the enduro graph) but its handler is the supermoto's.
/// - The off-road group is the other `enduro` bikes (touring, adventure, enduro) and the scooters.
/// - The road group is everything else: race and naked bikes, classics and cruisers (the bagger plays the naked set).
/// - A bike with no kind plays the rig's shared graph, which is the supermoto's, and takes the motard group with it.
pub fn class_of(kind: Option<&str>, handler: Option<&str>) -> BikeClass {
    let handler =
        handler.map(|name| name.trim_end_matches("_C").trim_end_matches("_InputHandler").to_ascii_lowercase());
    if handler.as_deref() == Some("motard") {
        return BikeClass::Motard;
    }
    match kind {
        None | Some("motard") => BikeClass::Motard,
        Some("enduro" | "scooter") => BikeClass::Offroad,
        Some(_) => BikeClass::Road,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_class_follows_the_graph_and_a_supermoto_handler() {
        let class = class_of;
        assert_eq!(class(Some("race"), Some("Default_InputHandler")), BikeClass::Road);
        assert_eq!(class(Some("naked"), Some("Naked_InputHandler_C")), BikeClass::Road);
        assert_eq!(class(Some("race"), None), BikeClass::Road);
        assert_eq!(class(Some("motard"), Some("Motard_InputHandler")), BikeClass::Motard);
        assert_eq!(class(Some("motard"), None), BikeClass::Motard);
        // The CRF 450 R SMT: an enduro type by the correction table, a supermoto by its handler.
        assert_eq!(class(Some("enduro"), Some("Motard_InputHandler")), BikeClass::Motard);
        assert_eq!(class(Some("enduro"), Some("Maxienduro_InputHandler")), BikeClass::Offroad);
        assert_eq!(class(Some("scooter"), Some("Scooter_InputHandler")), BikeClass::Offroad);
        assert_eq!(class(Some("enduro"), None), BikeClass::Offroad);
        // No kind: the shared graph, the supermoto's.
        assert_eq!(class(None, None), BikeClass::Motard);
        assert_eq!(class(Some("something-new"), None), BikeClass::Road);
    }
}
