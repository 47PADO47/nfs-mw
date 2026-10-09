//! Controls response rows; kept separate from the original option categories.

use super::options::{Data, Title};
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
        Title::Text(match self {
            Self::DeadzoneMode => "Deadzone Mode",
            Self::SteeringDeadzone => "Steering Deadzone",
            Self::CameraDeadzone => "Camera Deadzone",
            Self::TriggerDeadzone => "Trigger Deadzone",
            Self::SteeringSensitivity => "Steering Sensitivity",
            Self::CameraSensitivity => "Camera Sensitivity",
            Self::MouseSensitivity => "Mouse Sensitivity",
            Self::InvertCameraY => "Invert Camera Y",
        })
    }
    pub fn data(self, s: &Settings) -> Data {
        let value = match self {
            Self::DeadzoneMode => match s.controls.deadzone_mode {
                DeadzoneMode::Rescaled => "Rescaled".to_owned(),
                DeadzoneMode::Cutoff => "Cutoff".to_owned(),
            },
            Self::SteeringDeadzone => format!("{}%", s.controls.steering_deadzone),
            Self::CameraDeadzone => format!("{}%", s.controls.camera_deadzone),
            Self::TriggerDeadzone => format!("{}%", s.controls.trigger_deadzone),
            Self::SteeringSensitivity => format!("{}%", s.controls.steering_sensitivity),
            Self::CameraSensitivity => format!("{}%", s.controls.camera_sensitivity),
            Self::MouseSensitivity => format!("{}%", s.controls.mouse_sensitivity),
            Self::InvertCameraY => match s.controls.invert_camera_y {
                true => "On".to_owned(),
                false => "Off".to_owned(),
            },
        };
        Data::Text(value)
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
