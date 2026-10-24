//! The hidden `--hud-demo` value: a HUD state typed on the command line, for checking the HUD by eye. Its companion,
//! the hidden `--hud-scenario` value, names one of the deterministic fixtures of `devtools/hud_scenarios` instead.

pub(super) fn parse_hud_demo(s: &str) -> Result<crate::hud::HudState, String> {
    let n: Vec<f32> =
        s.split(',').map(|v| v.trim().parse::<f32>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let [speed, rpm, max_rpm, gear, rest @ ..] = &n[..] else { return Err(HUD_DEMO_FORMAT.into()) };
    let (speed, rpm, max_rpm, gear) = (*speed, *rpm, *max_rpm, *gear);
    if rest.len() > 8 {
        return Err(HUD_DEMO_FORMAT.into());
    }
    let red_line = max_rpm - 500.0;
    Ok(crate::hud::HudState {
        speed: speed / 3.6,
        rpm,
        max_rpm,
        red_line,
        gear: gear as i32,
        shift_light: rpm > red_line,
        has_nos: !rest.is_empty(),
        nos: rest.first().map_or(0.0, |p| p / 100.0),
        has_turbo: rest.len() > 1,
        boost_psi: rest.get(1).copied().unwrap_or(0.0),
        speedbreaker: rest.get(2).map_or(0.0, |percent| (percent / 100.0).clamp(0.0, 1.0)),
        speedbreaker_active: rest.get(3).is_some_and(|active| *active > 0.0),
        // A chase of this many seconds (none at 0), with this share of the evade done (the board's cool-down bar shows from 0).
        pursuit: rest.get(4).filter(|secs| **secs > 0.0).map(|secs| crate::hud::PursuitHud {
            secs: *secs,
            evade: rest.get(5).map_or(0.0, |share| (share / 100.0).clamp(0.0, 1.0)),
            cops_destroyed: 2,
            busted: rest.get(6).map_or(0.0, |percent| (percent / 100.0).clamp(-1.0, 1.0)),
            cooldown: rest.get(5).is_some_and(|share| *share > 0.0),
            // The hiding mark, with this share of its latch filled (none when left out).
            hiding: rest.get(8).map(|percent| (percent / 100.0).clamp(0.0, 1.0)),
        }),
        // A cop this many metres away, up the road a little to the left.
        radar: rest.get(7).map(|metres| crate::hud::RadarHud { angle: 0.35, distance: *metres }),
        // A chase has called five cars, two of them damaged.
        cops_engaged: if rest.get(4).is_some_and(|secs| *secs > 0.0) { 5 } else { 0 },
        cops_damaged: if rest.get(4).is_some_and(|secs| *secs > 0.0) { 2 } else { 0 },
        ..Default::default()
    })
}

pub(super) const HUD_DEMO_FORMAT: &str = "expected SPEED,RPM,MAX_RPM,GEAR[,NOS_PERCENT[,BOOST_PSI[,SPEEDBREAKER_PERCENT[,SPEEDBREAKER_ACTIVE[,PURSUIT_SECONDS[,EVADE_PERCENT[,BUSTED_PERCENT[,RADAR_METRES]]]]]]]]";

/// The `--hud-scenario` value: one of the names of `devtools/hud_scenarios` (free-roam, racing, cooldown and so on).
pub(super) fn parse_hud_scenario(s: &str) -> Result<crate::devtools::hud_scenarios::HudScenario, String> {
    s.parse()
}
