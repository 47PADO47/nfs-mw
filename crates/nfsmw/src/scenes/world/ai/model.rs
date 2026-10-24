use std::rc::Rc;

use nfsmw_data::car::physics::CarPhysics;

use crate::scenes::world::drive::CarRig;

/// A car model traffic can use: the rig is shared by every car of the model.
pub struct TrafficModel {
    pub name: String,
    pub rig: Rc<CarRig>,
    pub physics: CarPhysics,
    /// The saved tuning of a human racer's custom car; stock models use neutral tuning.
    pub tunings: blackbox_vehicle::Tunings,
    /// Its installed rigid-body behaviour is a tractor, whether or not it currently tows a trailer.
    pub tractor: bool,
    pub trailer: Option<Box<TrafficModel>>,
    pub cop_lights: Option<Rc<super::cop_lights::CopLightSet>>,
    pub audio_profile: crate::audio::traffic::TrafficProfile,
    pub keys: Rc<crate::scenes::world::keys::CarKeys>,
}
