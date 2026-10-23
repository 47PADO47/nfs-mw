//! The command line of the `nfsmw-server` binary, parsed by hand (a handful of flags).

use std::path::PathBuf;

/// The usage text.
pub const USAGE: &str = "\
nfsmw-server: the multiplayer server over the toy world (protocol tests, soak runs, CI).
The game serves its real world with `nfsmw --dedicated`.

USAGE: nfsmw-server [OPTIONS]

OPTIONS:
  --config <file>     the TOML config (docs/specs/multiplayer/server.md); defaults without it
  --port <n>          the game port, over the config's (0 lets the system pick)
  --toy-traffic <n>   traffic cars in the toy world (default 8)
  --toy-police <n>    police cars in the toy world (default 2)
  --bots <n>          bot clients driving in the session (needs the `bots` feature)
  --seconds <n>       stop after this many seconds
  --seed <n>          the toy world's seed (default: the clock)
  -h, --help          this text";

/// The parsed flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Args {
    pub config: Option<PathBuf>,
    pub port: Option<u16>,
    pub toy_traffic: Option<usize>,
    pub toy_police: Option<usize>,
    pub bots: usize,
    pub seconds: Option<u64>,
    pub seed: Option<u64>,
    pub help: bool,
}

/// Parses the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut out = Args::default();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        if flag == "-h" || flag == "--help" {
            out.help = true;
            continue;
        }
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--config" => out.config = Some(PathBuf::from(value)),
            "--port" => out.port = Some(number(&flag, &value)?),
            "--toy-traffic" => out.toy_traffic = Some(number(&flag, &value)?),
            "--toy-police" => out.toy_police = Some(number(&flag, &value)?),
            "--bots" => out.bots = number(&flag, &value)?,
            "--seconds" => out.seconds = Some(number(&flag, &value)?),
            "--seed" => out.seed = Some(number(&flag, &value)?),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(out)
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value.parse().map_err(|_| format!("{flag}: `{value}` is not a valid number"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn flags_parse() {
        let args = parse(strings(&["--config", "s.toml", "--toy-traffic", "40", "--seconds", "5", "--bots", "3"]))
            .expect("valid");
        assert_eq!(args.config, Some(PathBuf::from("s.toml")));
        assert_eq!(args.toy_traffic, Some(40));
        assert_eq!(args.seconds, Some(5));
        assert_eq!(args.bots, 3);
        assert!(parse(strings(&["--help"])).expect("valid").help);
    }

    #[test]
    fn bad_flags_are_refused() {
        assert!(parse(strings(&["--port"])).is_err());
        assert!(parse(strings(&["--port", "x"])).is_err());
        assert!(parse(strings(&["--colour", "red"])).is_err());
    }
}
