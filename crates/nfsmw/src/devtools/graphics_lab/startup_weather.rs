//! Startup time and weather for screenshots and benchmarks, which cannot click through the lab:
//! `NFSMW_LAB_WEATHER=hour=22,clouds=1,wet=1,rain=100`. Each key is optional; the values are the
//! ones the Environment tab sets (clock hour, cloud override 0 to 1, road wetness override 0 to 1,
//! rain particles in percent). Applied once, when the scene first reports its environment.
use super::*;

pub(super) fn apply(pending: &mut Option<String>, settings: &mut Settings, scene: &mut dyn crate::viewer::Scene) {
    if pending.is_none() {
        return;
    }
    let Some(mut value) = scene.environment() else {
        return;
    };
    let Some(text) = pending.take() else {
        return;
    };
    for part in text.split(',') {
        let Some((key, number)) = part.split_once('=') else {
            log::warn!("NFSMW_LAB_WEATHER: `{part}` is not key=value");
            continue;
        };
        let Ok(number) = number.trim().parse::<f32>() else {
            log::warn!("NFSMW_LAB_WEATHER: `{part}` has no number");
            continue;
        };
        match key.trim() {
            "hour" => value.hour = number.rem_euclid(24.0),
            "clouds" => value.cloud_override = Some(number.clamp(0.0, 1.0)),
            "wet" => value.road_wetness_override = Some(number.clamp(0.0, 1.0)),
            "rain" => {
                settings.always_rain = number > 0.0;
                settings.rain_amount = crate::settings::Percent(number.clamp(0.0, 100.0) as u8);
            }
            other => log::warn!("NFSMW_LAB_WEATHER: unknown key `{other}`"),
        }
    }
    value.settle_clouds = true;
    scene.set_environment(value);
}
