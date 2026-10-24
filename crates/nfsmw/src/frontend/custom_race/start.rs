//! What the custom race hands the front end to start a race.

/// What a custom race starts with.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomStart {
    /// The race's collection name.
    pub race: String,
    /// The car type folder under `CARS/`, and the preset it is built with.
    pub car: String,
    pub preset: Option<String>,
    /// My Cars identity: load the separately saved customization for this folder.
    pub custom: Option<String>,
    pub options: crate::scenes::world::RaceOptions,
    pub transmission: crate::settings::Transmission,
}
