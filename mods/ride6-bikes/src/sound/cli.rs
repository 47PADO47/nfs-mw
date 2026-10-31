//! The `sound` and `extract-bnk` commands of the importer (`docs/modding/ride6-bikes-sound.md`): decoded recordings in,
//! sound files and manifest tables out. `main.rs` hands the arguments after the command word to [`run`] and
//! [`extract_bnk`].
//!
//! ```text
//! sound --out <mod folder> --car-id KTM450 --idle 2300 --redline 11000
//!       [--data resolved.json]                          the bike's resolved audio data (backfire numbers)
//!       --layer "<rpm>:<wav>[*gain],<wav>[*gain]" ...   one per engine loop
//!       [--backfire <wav>]... [--shift-up <wav>]... [--shift-down <wav>]... [--pops <wav>]...
//!       [--no-overrun-table]                            leave [car_sound.overrun] out (the core backfire rule)
//!       [--mixer-shifts]                                leave `shift_trigger` out (shifts at the engine mixer's clunk)
//! extract-bnk <bank.bnk> <out folder>                   write the media of a bank as <media id>.wem
//! ```
//!
//! `sound` writes `<out>/files/CARS/<ID>/SOUND/*.WAV` and `<out>/cars/<id>.sound.toml` (the two tables, to be pasted
//! into or appended to the car's manifest) and prints the tables. With `--data` the table
//! `[car_sound.overrun]` is written when the bike backfires by the accumulator system.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use super::data::BikeAudioData;
use super::plan::{self, EngineLayerSource, SoundOptions, SoundSources};

fn value<'a>(args: &'a [String], at: usize, flag: &str) -> Result<&'a str> {
    args.get(at + 1).map(String::as_str).with_context(|| format!("{flag} needs a value"))
}

fn number(text: &str, flag: &str) -> Result<f32> {
    text.parse().with_context(|| format!("{flag}: `{text}` is not a number"))
}

/// Run the `sound` command on `args` (the arguments after the word `sound`).
pub fn run(args: &[String]) -> Result<()> {
    let mut sources = SoundSources::default();
    let (mut out, mut car_id, mut data) = (None, None, None);
    let (mut idle, mut redline) = (None, None);
    let (mut overrun_table, mut mixer_shifts) = (true, false);
    let mut at = 0;
    while at < args.len() {
        let flag = args[at].as_str();
        if flag == "--no-overrun-table" {
            overrun_table = false;
            at += 1;
            continue;
        }
        if flag == "--mixer-shifts" {
            mixer_shifts = true;
            at += 1;
            continue;
        }
        let item = value(args, at, flag)?;
        match flag {
            "--out" => out = Some(PathBuf::from(item)),
            "--car-id" => car_id = Some(item.to_string()),
            "--data" => data = Some(PathBuf::from(item)),
            "--idle" => idle = Some(number(item, flag)?),
            "--redline" => redline = Some(number(item, flag)?),
            "--layer" => sources.engine_layers.push(EngineLayerSource::parse(item)?),
            "--backfire" => sources.backfire.push(PathBuf::from(item)),
            "--shift-up" => sources.shift_up.push(PathBuf::from(item)),
            "--shift-down" => sources.shift_down.push(PathBuf::from(item)),
            "--pops" => sources.pops.push(PathBuf::from(item)),
            other => bail!("unknown option `{other}`"),
        }
        at += 2;
    }
    let (Some(out), Some(car_id), Some(idle), Some(redline)) = (out, car_id, idle, redline) else {
        bail!("--out, --car-id, --idle and --redline are required");
    };
    let mut options = SoundOptions::new(car_id.clone(), idle, redline);
    options.shift_on_gear = !mixer_shifts;
    if let Some(path) = data.filter(|_| overrun_table) {
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let params = BikeAudioData::from_json(&text)?.overrun()?;
        options.overrun = params.applies().then_some(params);
        if options.overrun.is_none() {
            println!(
                "note: this bike's backfire simulation is off or uses the old system: no [car_sound.overrun] table"
            );
        }
    }
    let sound = plan::build(&sources, &options)?;
    plan::write_files(&sound, &out.join("files"))?;
    let tables = out.join("cars").join(format!("{}.sound.toml", car_id.to_ascii_lowercase()));
    if let Some(parent) = tables.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = sound.manifest_text();
    fs::write(&tables, &text).with_context(|| format!("writing {}", tables.display()))?;
    println!("{text}");
    println!(
        "wrote {} sound files under {} and the tables to {}",
        sound.files.len(),
        out.join("files").display(),
        tables.display()
    );
    Ok(())
}

/// Run the `extract-bnk <bank.bnk> <out folder>` command: write every embedded recording as `<media id>.wem`.
pub fn extract_bnk(args: &[String]) -> Result<()> {
    let [bank, out] = args else { bail!("usage: extract-bnk <bank.bnk> <out folder>") };
    let bytes = fs::read(bank).with_context(|| format!("reading {bank}"))?;
    let bank = super::bnk::parse(&bytes)?;
    fs::create_dir_all(out).with_context(|| format!("creating {out}"))?;
    for media in &bank.media {
        let path = PathBuf::from(out).join(format!("{}.wem", media.id));
        fs::write(&path, &media.bytes).with_context(|| format!("writing {}", path.display()))?;
    }
    println!("bank {} (version {}): wrote {} media to {out}", bank.bank_id, bank.version, bank.media.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::wav::{self, Pcm};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-cli-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn tone(dir: &std::path::Path, name: &str) -> String {
        let samples = (0..4000).map(|i| (i as f32 * 0.2).sin() * 0.5).collect();
        let path = dir.join(name);
        fs::write(&path, wav::encode16(&Pcm { sample_rate: 8000, channels: 1, samples, loop_range: None })).unwrap();
        path.to_string_lossy().into_owned()
    }

    const DATA: &str = r#"{
      "chassis": "X", "variant": "Custom", "data_asset": "/Game/a/B", "events": {},
      "audio_effects_default": { "BikeEffectsParametersValue": {
        "bBackfireSimulationEnabled": false, "bNewBackfireSystemEnabled": false,
        "AccumulatorDeltaFactor": 0.2, "BackfireEffectThrottleInitialThreshold": 0.9,
        "BackfireEffectThrottleTriggerThreshold": 0.1, "BackfireMaximumDeltaTime": 2.0,
        "FlameEffectDelayTime": 0.2, "FlameEffectMinRPM": 0.1,
        "BackfireBangAudiotIntensityThreshold": 0.85, "BackfireBangParticlesIntensityThreshold": 0.95,
        "BackfireFlameParticlesIntensityThreshold": 0.7,
        "BackfireBangProbabilityRange": {"X": 0.8, "Y": 0.3} } },
      "audio_effects_override": { "BikeEffectsParametersValue": {
        "bBackfireSimulationEnabled": true, "bNewBackfireSystemEnabled": true } }
    }"#;

    #[test]
    fn the_sound_command_writes_the_files_and_the_tables_with_the_overrun_and_the_gear_trigger() {
        let dir = temp("run");
        let layer = format!("3000:{}", tone(&dir, "a.wav"));
        let data = dir.join("data.json");
        fs::write(&data, DATA).unwrap();
        let out = dir.join("mod");
        let bang = tone(&dir, "bang.wav");
        run(&args(&[
            "--out",
            out.to_str().unwrap(),
            "--car-id",
            "T1",
            "--idle",
            "2300",
            "--redline",
            "11000",
            "--layer",
            layer.as_str(),
            "--backfire",
            bang.as_str(),
            "--data",
            data.to_str().unwrap(),
        ]))
        .unwrap();
        assert!(out.join("files/CARS/T1/SOUND/ENG0.WAV").is_file());
        assert!(out.join("files/CARS/T1/SOUND/BACKFIRE0.WAV").is_file());
        let text = fs::read_to_string(out.join("cars/t1.sound.toml")).unwrap();
        assert!(text.contains("shift_trigger = \"gear\"") && text.contains("[car_sound.overrun]"));
        assert!(text.contains("bang_audio_threshold = 0.85"));
        // The opt-outs.
        let plain = dir.join("plain");
        run(&args(&[
            "--out",
            plain.to_str().unwrap(),
            "--car-id",
            "T1",
            "--idle",
            "2300",
            "--redline",
            "11000",
            "--layer",
            layer.as_str(),
            "--data",
            data.to_str().unwrap(),
            "--no-overrun-table",
            "--mixer-shifts",
        ]))
        .unwrap();
        let text = fs::read_to_string(plain.join("cars/t1.sound.toml")).unwrap();
        assert!(!text.contains("shift_trigger") && !text.contains("overrun"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_values_and_unknown_options_are_refused() {
        assert!(run(&args(&["--car-id", "T1"])).is_err());
        assert!(run(&args(&["--bogus", "1"])).is_err());
        assert!(run(&args(&["--out"])).is_err());
    }

    #[test]
    fn extract_bnk_writes_one_wem_per_media() {
        let dir = temp("bnk");
        let mut header = Vec::new();
        for value in [145u32, 3] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        let mut index = Vec::new();
        for value in [40u32, 0, 3, 41, 3, 2] {
            index.extend_from_slice(&value.to_le_bytes());
        }
        let mut bank = Vec::new();
        for (tag, body) in [(b"BKHD", header), (b"DIDX", index), (b"DATA", b"abcde".to_vec())] {
            bank.extend_from_slice(tag);
            bank.extend_from_slice(&(body.len() as u32).to_le_bytes());
            bank.extend_from_slice(&body);
        }
        let file = dir.join("t.bnk");
        fs::write(&file, bank).unwrap();
        let out = dir.join("wem");
        extract_bnk(&args(&[file.to_str().unwrap(), out.to_str().unwrap()])).unwrap();
        assert_eq!(fs::read(out.join("40.wem")).unwrap(), b"abc");
        assert_eq!(fs::read(out.join("41.wem")).unwrap(), b"de");
        assert!(extract_bnk(&args(&["only-one"])).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
