//! Value parsers for the command-line options that are not plain numbers or words.

use crate::settings::Resolution;

pub(super) fn parse_xy(s: &str) -> Result<[f32; 2], String> {
    let (x, y) = s.split_once(',').ok_or("expected X,Y")?;
    let n = |v: &str| v.trim().parse::<f32>().map_err(|e| e.to_string());
    Ok([n(x)?, n(y)?])
}

pub(super) fn parse_screenshot_size(s: &str) -> Result<[u32; 2], String> {
    let Resolution::Pixels { width, height } = s.parse::<Resolution>()? else {
        return Err("expected WIDTHxHEIGHT, not native".into());
    };
    Ok([width, height])
}
