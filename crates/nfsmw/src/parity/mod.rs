//! Options for unattended, repeatable comparison runs against the original game: a seed for the random sources that
//! single player otherwise seeds from the clock (`seed.rs`), the switch that forces every mechanics-changing setting to
//! the original's behaviour (`stock.rs`), the header a trace records about the run (`header.rs`), and the unattended
//! race run: its options (`race_run.rs`) and its record (`race_trace.rs`); and the career-flow trace (`career_trace.rs`).
//!
//! The options are per run: the command line or the environment sets them once, before anything reads them, and nothing
//! saves them.

pub mod career_trace;
pub mod header;
pub mod nis_capture;
pub mod race_run;
pub mod race_trace;
pub mod seed;
pub mod stock;

use std::sync::OnceLock;

/// The variable that seeds a run, as `--seed` does.
pub const SEED_ENV: &str = "NFSMW_SEED";
/// The variable that turns the stock rules on, as `--stock-rules` does.
pub const STOCK_RULES_ENV: &str = "NFSMW_STOCK_RULES";

/// What a run was started with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParityOptions {
    /// The seed of the random sources single player seeds from the clock. `None` keeps the clock.
    pub seed: Option<u64>,
    /// Every mechanics-changing setting is forced to the original game's behaviour (`stock.rs`).
    pub stock_rules: bool,
    /// The vehicle budget the command line asked for (`--traffic N`, `--no-traffic` as 0). A stock run keeps it over
    /// its own budget: the original can run a race without traffic.
    pub traffic: Option<u32>,
}

impl ParityOptions {
    /// The command line over the environment, read through `get`. A value that does not parse is reported and ignored.
    pub fn resolve(seed: Option<u64>, stock_rules: bool, get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            seed: seed
                .or_else(|| env_value(&get, SEED_ENV, |text| text.trim().parse::<u64>().map_err(|e| e.to_string()))),
            stock_rules: stock_rules || env_value(&get, STOCK_RULES_ENV, crate::settings::parse_bool).unwrap_or(false),
            traffic: None,
        }
    }
}

fn env_value<T>(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Option<T> {
    let text = get(name)?;
    match parse(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            log::warn!("ignoring {name}: {e}");
            None
        }
    }
}

static OPTIONS: OnceLock<ParityOptions> = OnceLock::new();

/// Sets the run's options. Called once, when the command starts; a later call changes nothing.
pub fn init(options: ParityOptions) {
    if OPTIONS.set(options).is_err() {
        return;
    }
    if let Some(seed) = options.seed {
        log::info!("run seed {seed}");
    }
    if options.stock_rules {
        log::info!("stock rules: mechanics-changing settings are forced to the original's behaviour");
    }
}

/// The run's options: none unless [`init`] set them.
pub fn options() -> ParityOptions {
    OPTIONS.get().copied().unwrap_or_default()
}

/// Whether the run forces the original's mechanics.
pub fn stock_rules() -> bool {
    options().stock_rules
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
    }

    #[test]
    fn nothing_given_changes_nothing() {
        assert_eq!(ParityOptions::resolve(None, false, env(&[])), ParityOptions::default());
        assert_eq!(ParityOptions::default(), ParityOptions { seed: None, stock_rules: false, traffic: None });
    }

    #[test]
    fn the_environment_sets_both() {
        let options = ParityOptions::resolve(None, false, env(&[(SEED_ENV, " 42 "), (STOCK_RULES_ENV, "1")]));
        assert_eq!(options, ParityOptions { seed: Some(42), stock_rules: true, traffic: None });
    }

    #[test]
    fn the_command_line_wins_and_bad_values_are_ignored() {
        let options = ParityOptions::resolve(Some(7), false, env(&[(SEED_ENV, "42"), (STOCK_RULES_ENV, "0")]));
        assert_eq!(options, ParityOptions { seed: Some(7), stock_rules: false, traffic: None });
        let bad = ParityOptions::resolve(None, false, env(&[(SEED_ENV, "-3"), (STOCK_RULES_ENV, "perhaps")]));
        assert_eq!(bad, ParityOptions::default());
        assert!(ParityOptions::resolve(None, true, env(&[(STOCK_RULES_ENV, "off")])).stock_rules);
    }
}
