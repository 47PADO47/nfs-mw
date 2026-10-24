//! Controls response rows; kept separate from the original option categories.

use super::ids::{LABEL_OFF, LABEL_ON};
use super::options::{Data, Title, message};
use crate::settings::{DeadzoneMode, Partial, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputSetting {
    DeadzoneMode,
    SteeringDeadzone,
    CameraDeadzone,
    TriggerDeadzone,
    SteeringSensitivity,
    CameraSensitivity,
    MouseSensitivity,
    InvertCameraY,
}

impl InputSetting {
    pub const ALL: [Self; 8] = [
        Self::DeadzoneMode,
        Self::SteeringDeadzone,
        Self::CameraDeadzone,
        Self::TriggerDeadzone,
        Self::SteeringSensitivity,
        Self::CameraSensitivity,
        Self::MouseSensitivity,
        Self::InvertCameraY,
    ];
    pub fn title(self) -> Title {
        Title::Key(match self {
            Self::DeadzoneMode => "options-deadzone-mode",
            Self::SteeringDeadzone => "options-steering-deadzone",
            Self::CameraDeadzone => "options-camera-deadzone",
            Self::TriggerDeadzone => "options-trigger-deadzone",
            Self::SteeringSensitivity => "options-steering-sensitivity",
            Self::CameraSensitivity => "options-camera-sensitivity",
            Self::MouseSensitivity => "options-mouse-sensitivity",
            Self::InvertCameraY => "options-invert-camera-y",
        })
    }
    pub fn data(self, s: &Settings) -> Data {
        let controls = &s.controls;
        let percent = match self {
            Self::DeadzoneMode => {
                return match controls.deadzone_mode {
                    DeadzoneMode::Rescaled => message("options-deadzone-rescaled"),
                    DeadzoneMode::Cutoff => message("options-deadzone-cutoff"),
                };
            }
            Self::InvertCameraY => {
                return Data::Label(match controls.invert_camera_y {
                    true => LABEL_ON,
                    false => LABEL_OFF,
                });
            }
            Self::SteeringDeadzone => i64::from(controls.steering_deadzone.percent()),
            Self::CameraDeadzone => i64::from(controls.camera_deadzone.percent()),
            Self::TriggerDeadzone => i64::from(controls.trigger_deadzone.percent()),
            Self::SteeringSensitivity => i64::from(controls.steering_sensitivity.percent()),
            Self::CameraSensitivity => i64::from(controls.camera_sensitivity.percent()),
            Self::MouseSensitivity => i64::from(controls.mouse_sensitivity.percent()),
        };
        Data::Text(crate::i18n::fmt::percent(percent))
    }
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        let before = s.controls;
        match self {
            Self::DeadzoneMode => {
                s.controls.deadzone_mode = match s.controls.deadzone_mode {
                    DeadzoneMode::Rescaled => DeadzoneMode::Cutoff,
                    DeadzoneMode::Cutoff => DeadzoneMode::Rescaled,
                };
                changed.deadzone_mode = Some(s.controls.deadzone_mode);
            }
            Self::SteeringDeadzone => {
                s.controls.steering_deadzone = s.controls.steering_deadzone.step(forward);
                changed.steering_deadzone = Some(s.controls.steering_deadzone);
            }
            Self::CameraDeadzone => {
                s.controls.camera_deadzone = s.controls.camera_deadzone.step(forward);
                changed.camera_deadzone = Some(s.controls.camera_deadzone);
            }
            Self::TriggerDeadzone => {
                s.controls.trigger_deadzone = s.controls.trigger_deadzone.step(forward);
                changed.trigger_deadzone = Some(s.controls.trigger_deadzone);
            }
            Self::SteeringSensitivity => {
                s.controls.steering_sensitivity = s.controls.steering_sensitivity.step(forward);
                changed.steering_sensitivity = Some(s.controls.steering_sensitivity);
            }
            Self::CameraSensitivity => {
                s.controls.camera_sensitivity = s.controls.camera_sensitivity.step(forward);
                changed.camera_sensitivity = Some(s.controls.camera_sensitivity);
            }
            Self::MouseSensitivity => {
                s.controls.mouse_sensitivity = s.controls.mouse_sensitivity.step(forward);
                changed.mouse_sensitivity = Some(s.controls.mouse_sensitivity);
            }
            Self::InvertCameraY => {
                s.controls.invert_camera_y = !s.controls.invert_camera_y;
                changed.invert_camera_y = Some(s.controls.invert_camera_y);
            }
        }
        before != s.controls
    }
}
