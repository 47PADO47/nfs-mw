//! The `exhaust-flames` console command and the status line.

use super::trigger::blowoff_allowed;
use super::{EMITTER_LIMIT, ExhaustFlames, State};

const USAGE: &str = "usage: exhaust-flames [status|engine <level>|pop]";

fn word(on: bool) -> &'static str {
    match on {
        true => "on",
        false => "off",
    }
}

impl ExhaustFlames {
    /// Fake one sputter pop: a backfire at the next step, whatever the throttle does.
    pub fn pop(&mut self) {
        self.forced = self.enabled;
    }

    /// `exhaust-flames [status|engine <level>|pop]`. The flames themselves are switched with the
    /// `exhaust_flames` setting (`set exhaust_flames off`).
    pub fn command(&mut self, args: &[&str]) -> Result<String, String> {
        match args {
            [] | ["status"] => {}
            ["pop"] => {
                if !self.enabled {
                    return Err("the exhaust flames are off (set exhaust_flames on)".into());
                }
                self.pop();
            }
            ["engine", level] => {
                self.engine_level = level.parse::<i32>().ok().filter(|l| *l >= 0).ok_or(USAGE)?;
            }
            _ => return Err(USAGE.into()),
        }
        Ok(self.status())
    }

    pub fn status(&self) -> String {
        let State::Ready(active) = &self.state else {
            let why = match (self.enabled, &self.state) {
                (false, _) => "nothing loaded",
                (true, State::Missing) => "this car has no flame data",
                (true, _) => "loads at the next frame",
            };
            return format!("exhaust flames {}: {why}, engine level {}", word(self.enabled), self.engine_level);
        };
        format!(
            "exhaust flames {}: {} pipes, engine level {} of {} (shift blow-off {}), {}/{} particles, {}",
            word(self.enabled),
            active.pipes.len(),
            self.engine_level,
            active.engine_upgrades,
            word(blowoff_allowed(self.engine_level, active.engine_upgrades)),
            active.live,
            active.pipes.len() * active.lanes.len() * EMITTER_LIMIT,
            match active.intensity > 0.0 {
                true => "flaming",
                false => "quiet",
            }
        )
    }
}
