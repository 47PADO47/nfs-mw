//! The hidden `--hud-demo` value: a HUD state typed on the command line, for checking the HUD by eye.

pub(super) fn parse_hud_demo(s: &str) -> Result<crate::hud::HudState, String> {
    let n: Vec<f32> =
        s.split(',').map(|v| v.trim().parse::<f32>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let [speed, rpm, max_rpm, gear, rest @ ..] = &n[..] else { return Err(HUD_DEMO_FORMAT.into()) };
    let (speed, rpm, max_rpm, gear) = (*speed, *rpm, *max_rpm, *gear);
    if rest.len() > 4 {
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
        ..Default::default()
    })
}

pub(super) const HUD_DEMO_FORMAT: &str =
    "expected SPEED,RPM,MAX_RPM,GEAR[,NOS_PERCENT[,BOOST_PSI[,SPEEDBREAKER_PERCENT[,SPEEDBREAKER_ACTIVE]]]]";
