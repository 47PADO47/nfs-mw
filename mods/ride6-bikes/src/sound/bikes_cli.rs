//! The `sound-bikes` command: build the sound of every imported bike from the user's extracted RIDE 6 banks.
//!
//! ```text
//! sound-bikes --bikes <exporter output> --physics <physics dump> --resolved <audiomap folder> --wwise <ride6/Content>
//!             --out <sound folder> [--vgmstream <vgmstream-cli>] [--variant Custom|Stock] [--only <vehicle id>]...
//!             [--report <file.md>] [--work <scratch folder>]
//! ```
//!
//! For each bike folder of `--bikes` (the car ids are assigned to the whole set, as the batch import does) it reads the
//! engine's idle and limiter speed from the bike's physics file, the bike's audio data from `--resolved`
//! (`<chassis>__<variant>.json`, written by `pak-sound`) and builds the sound ([`super::bike_sound`]). It writes
//! `<out>/files/SOUND/RIDE6/...` (shared recordings) and `<out>/cars/<id>.sound.toml` (the tables, read by the importer's
//! `sound` setting), and a report that lists every bike left without its own engine sound, with the reason.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;

use super::bike_sound::{BikeSoundOut, Builder, resolved_file};
use super::data::BikeAudioData;
use super::decode::Vgmstream;
use super::wwise::Wwise;
use crate::bike_table;
use crate::naming;
use crate::partfile::PartTable;

#[derive(Parser)]
#[command(about = "Build the sound of every imported bike from RIDE 6's extracted sound banks")]
struct Args {
    /// The exporter's output: one folder per bike, each with a `parts.json`.
    #[arg(long)]
    bikes: PathBuf,
    /// The physics dump (for each engine's idle and limiter speed).
    #[arg(long)]
    physics: PathBuf,
    /// The folder `pak-sound --resolved` wrote.
    #[arg(long)]
    resolved: PathBuf,
    /// The folder that contains `WwiseAudio/` (`<pak-sound --out>/ride6/Content`).
    #[arg(long)]
    wwise: PathBuf,
    /// The output folder: `files/` and `cars/` are written in it.
    #[arg(long)]
    out: PathBuf,
    /// `vgmstream-cli` (default: $VGMSTREAM_CLI, else on the path).
    #[arg(long)]
    vgmstream: Option<PathBuf>,
    /// Which audio variant of a chassis to use: `Custom` (the aftermarket exhaust) or `Stock`.
    #[arg(long, default_value = "Custom")]
    variant: String,
    /// Only these vehicle ids.
    #[arg(long)]
    only: Vec<String>,
    /// The report file (default: `<out>/sound-report.md`).
    #[arg(long)]
    report: Option<PathBuf>,
    /// The scratch folder for the decoder (default: `<out>/.work`).
    #[arg(long)]
    work: Option<PathBuf>,
}

/// The engine's idle and limiter speed from the bike's resolved physics file.
fn engine_speeds(physics: &Path, physics_name: &str) -> Result<(f32, f32)> {
    let file = bike_table::locate(physics, physics_name)?;
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let doc: Value = serde_json::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
    let engine = doc.pointer("/Properties/Engine").with_context(|| format!("{} has no Engine", file.display()))?;
    let number = |key: &str| engine.get(key).and_then(Value::as_f64);
    let idle = number("MinRPM").or_else(|| number("MinRPMLimit")).context("the engine has no MinRPM")?;
    let max = number("MaxRPM").context("the engine has no MaxRPM")?;
    Ok((idle as f32, max as f32))
}

/// The resolved audio data of a vehicle: the wanted variant, else the other one.
fn audio_data(dir: &Path, vehicle: &str, variant: &str) -> Result<(BikeAudioData, String)> {
    let other = if variant == "Custom" { "Stock" } else { "Custom" };
    for name in [variant, other] {
        let file = resolved_file(dir, vehicle, name);
        if !file.is_file() {
            continue;
        }
        let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        return Ok((BikeAudioData::from_json(&text)?, name.to_owned()));
    }
    anyhow::bail!("RIDE 6 has no audio data row for the chassis `{vehicle}`")
}

struct Row {
    vehicle: String,
    car: String,
    result: std::result::Result<(BikeSoundOut, String), String>,
}

fn one_bike(
    builder: &mut Builder,
    args: &Args,
    vehicle: &str,
    physics_name: &str,
    car: &str,
) -> Result<(BikeSoundOut, String)> {
    let (data, variant) = audio_data(&args.resolved, vehicle, &args.variant)?;
    let (idle, redline) = engine_speeds(&args.physics, physics_name)?;
    let out = builder.bike(&data, idle, redline)?;
    let tables = args.out.join("cars").join(format!("{}.sound.toml", car.to_ascii_lowercase()));
    if let Some(parent) = tables.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&tables, &out.tables).with_context(|| format!("writing {}", tables.display()))?;
    Ok((out, variant))
}

fn size_of(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => size_of(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

fn report(rows: &[Row], args: &Args) -> String {
    let mut text = String::from("# RIDE 6 bike sound report\n\n");
    let built: Vec<&Row> = rows.iter().filter(|row| row.result.is_ok()).collect();
    let _ = writeln!(text, "Bikes: {} with their own engine sound, {} without.", built.len(), rows.len() - built.len());
    let mut engines: Vec<&str> =
        built.iter().filter_map(|row| row.result.as_ref().ok()).map(|(out, _)| out.engine_key.as_str()).collect();
    engines.sort_unstable();
    engines.dedup();
    let files = args.out.join("files");
    let _ = writeln!(
        text,
        "Distinct engines (shared sets): {}. Recordings written under `files/`: {:.1} MiB.\n",
        engines.len(),
        size_of(&files) as f64 / 1_048_576.0
    );
    text.push_str("## Without their own sound (the built-in engine plays)\n\n");
    for row in rows.iter().filter(|row| row.result.is_err()) {
        if let Err(reason) = &row.result {
            let _ = writeln!(text, "- `{}` ({}): {reason}", row.vehicle, row.car);
        }
    }
    text.push_str("\n## With their own sound\n\n| Vehicle | Car id | Variant | Engine set | Layers | Shift up / down | Backfire | Pops | Worst detune | Notes |\n|---|---|---|---|---|---|---|---|---|---|\n");
    for row in &built {
        let Ok((out, variant)) = &row.result else { continue };
        let [up, down, bang, pops] = out.effects;
        let _ = writeln!(
            text,
            "| {} | {} | {variant} | {} | {} | {up} / {down} | {bang} | {pops} | {:.1}% | {} |",
            row.vehicle,
            row.car,
            out.engine_key,
            out.layers,
            out.worst_detune * 100.0,
            out.notes.join("; ")
        );
    }
    text
}

/// Run the command on the arguments after the word `sound-bikes`.
pub fn run(raw: &[String]) -> Result<()> {
    let args = Args::try_parse_from(std::iter::once("sound-bikes".to_owned()).chain(raw.iter().cloned()))?;
    let (found, unreadable) = crate::batch::discover(&args.bikes)?;
    let vehicles: Vec<String> = found.iter().map(|bike| bike.vehicle.clone()).collect();
    let ids = naming::assign(&vehicles);
    let work = args.work.clone().unwrap_or_else(|| args.out.join(".work"));
    let mut builder = Builder {
        wwise: Wwise::new(&args.wwise)?,
        vgmstream: Vgmstream::new(args.vgmstream.as_deref(), &work)?,
        files: args.out.join("files"),
    };
    fs::create_dir_all(&args.out).with_context(|| format!("creating {}", args.out.display()))?;
    let mut rows = Vec::new();
    for (name, reason) in unreadable {
        rows.push(Row { vehicle: name, car: String::new(), result: Err(format!("parts.json: {reason}")) });
    }
    for (bike, car) in found.iter().zip(&ids) {
        if !args.only.is_empty() && !args.only.contains(&bike.vehicle) {
            continue;
        }
        let physics_name = PartTable::read(&bike.dir).map(|table| table.physics_name());
        let result = match physics_name {
            Ok(name) => one_bike(&mut builder, &args, &bike.vehicle, &name, car).map_err(|error| format!("{error:#}")),
            Err(error) => Err(format!("{error:#}")),
        };
        match &result {
            Ok((out, variant)) => {
                println!("{car}: {variant} sound, {} layers, engine set {}", out.layers, out.engine_key)
            }
            Err(reason) => println!("{car}: no sound: {reason}"),
        }
        rows.push(Row { vehicle: bike.vehicle.clone(), car: car.clone(), result });
    }
    let text = report(&rows, &args);
    let path = args.report.clone().unwrap_or_else(|| args.out.join("sound-report.md"));
    fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
    let _ = fs::remove_dir_all(&work);
    println!("report: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_speeds_come_from_the_resolved_physics_file() {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-speeds-{}", std::process::id()));
        fs::create_dir_all(dir.join("vehicles/KTM")).unwrap();
        fs::write(
            dir.join("vehicles/KTM/KTM_450SMR_2022.json"),
            r#"{"Properties":{"Engine":{"MinRPM":2300.0,"MaxRPM":11000.0}}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("vehicles/KTM/KTM_X_2000.json"),
            r#"{"Properties":{"Engine":{"MinRPMLimit":1500.0,"MaxRPM":9000.0}}}"#,
        )
        .unwrap();
        fs::write(dir.join("vehicles/KTM/KTM_Y_2000.json"), r#"{"Properties":{}}"#).unwrap();
        assert_eq!(engine_speeds(&dir, "KTM_450SMR_2022").unwrap(), (2300.0, 11000.0));
        assert_eq!(engine_speeds(&dir, "KTM_X_2000").unwrap(), (1500.0, 9000.0));
        assert!(engine_speeds(&dir, "KTM_Y_2000").is_err());
        assert!(engine_speeds(&dir, "NOBODY").is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_chassis_without_a_row_is_an_error_and_the_other_variant_is_the_fallback() {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-audio-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let json = r#"{"chassis":"X","variant":"Stock","data_asset":"/a","events":{},"audio_effects_default":{},"audio_effects_override":null}"#;
        fs::write(dir.join("X__Stock.json"), json).unwrap();
        let (data, variant) = audio_data(&dir, "X", "Custom").unwrap();
        assert_eq!((data.variant.as_str(), variant.as_str()), ("Stock", "Stock"));
        assert!(audio_data(&dir, "Y", "Custom").is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_report_lists_missing_sounds_with_the_reason() {
        let args = Args::try_parse_from([
            "x",
            "--bikes",
            "a",
            "--physics",
            "b",
            "--resolved",
            "c",
            "--wwise",
            "d",
            "--out",
            "nowhere-ride6",
        ])
        .unwrap();
        let rows = vec![Row { vehicle: "Z_1".into(), car: "Z_1".into(), result: Err("no bank".into()) }];
        let text = report(&rows, &args);
        assert!(text.contains("0 with their own engine sound, 1 without"));
        assert!(text.contains("- `Z_1` (Z_1): no bank"));
    }
}
