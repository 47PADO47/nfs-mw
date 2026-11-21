//! The model of the car being driven: everything the three hooks keep between calls, built once per car change.

use crate::aids::AidCar;
use crate::controls::ControlsState;
use crate::grip::{GripScale, NoThermal, ThermalGrip};
use crate::mapping::{CarParams, CarSpec};
use crate::suspension::{AxleCfg, AxleState};
use crate::thermal::ThermalParams;
use crate::wheel::WheelState;

/// The tyres, hubs, aids and thermal state of one car.
pub struct Model {
    /// The parameter set of the car.
    pub params: CarParams,
    /// The id of the car (`spec-<kg>-<cm>-<kW>`).
    pub name: String,
    /// The tyre state of the four wheels.
    pub wheels: [WheelState; 4],
    /// The thermal grip: [`ThermalGrip`] with the thermal unit on, [`NoThermal`] without.
    pub thermal: Box<dyn GripScale>,
    /// Whether the thermal unit is the one in `thermal`.
    pub thermal_on: bool,
    /// The suspension of the front and the rear axle.
    pub axle_cfg: [AxleCfg; 2],
    /// The hubs of the front and the rear axle.
    pub axles: [AxleState; 2],
    /// The aids' constants for the car, when its numbers allow them.
    pub aid_car: Option<AidCar>,
    /// The controls hook's state.
    pub controls: ControlsState,
    /// The camber the last suspension answer gave each wheel (outward positive), for the HUD.
    pub camber: [f64; 4],
}

impl Model {
    /// The model of the car `spec` describes with the parameters `params`.
    pub fn new(spec: &CarSpec, params: CarParams, name: String) -> Model {
        let track = [spec.track_m[0], spec.track_m[1]];
        let axle_cfg = [
            AxleCfg { keys: params.suspension[0].clone(), track: track[0] },
            AxleCfg { keys: params.suspension[1].clone(), track: track[1] },
        ];
        Model {
            params,
            name,
            wheels: [WheelState::fresh(); 4],
            thermal: Box::new(NoThermal),
            thermal_on: false,
            axle_cfg,
            axles: [AxleState::new(), AxleState::new()],
            aid_car: AidCar::from_spec(spec),
            controls: ControlsState::default(),
            camber: [0.0; 4],
        }
    }

    /// Switch the thermal unit on or off: a new set of tyres at the ambient temperature (or the warm one), or none.
    pub fn set_thermal(&mut self, on: bool, warm_start: bool, ambient_c: f64) {
        self.thermal_on = on;
        if !on {
            self.thermal = Box::new(NoThermal);
            return;
        }
        let mut grip = ThermalGrip::new(ThermalParams::default(), warm_start);
        grip.reset(ambient_c);
        self.thermal = Box::new(grip);
    }

    /// Start everything cold: fresh tyre slip state, new hubs, the aids and the steering from nothing, the thermal state reset.
    pub fn reseed(&mut self, ambient_c: f64) {
        self.wheels = [WheelState::fresh(); 4];
        self.axles = [AxleState::new(), AxleState::new()];
        self.controls.reset();
        self.thermal.reset(ambient_c);
    }
}
