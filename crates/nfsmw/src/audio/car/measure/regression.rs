//! The numeric thresholds a converted 2012 car must keep against the cars of the 2005 game, through the real headless
//! mixer. The numbers come from `docs/modding/car-sound.md` ("Measured against the cars of the game"): the stock cars'
//! engine is -17 to -22 dB over a pull and within 9 dB of itself, never louder than -6.5 dB at the peak, and the
//! converted cars used to be 40 dB apart at idle, 15 dB apart at the limiter, and hissing (centroid above 3 kHz).
//!
//! Ignored without the installs: `NFSMW_GAME_DIR` (the 2005 install) and `NFSMW_MEASURE_MOD` (the converted cars).

use std::path::PathBuf;

use super::report::{self, Metrics, STEPS, steady_key};
use super::rig::Rig;

/// The cars measured: BMW M3 GTR is the reference, then every converted car (`NFSMW_MEASURE_CARS` narrows them).
fn converted() -> Vec<String> {
    if let Ok(list) = std::env::var("NFSMW_MEASURE_CARS") {
        return list.split(',').map(|name| name.trim().to_owned()).filter(|name| !name.is_empty()).collect();
    }
    nfsmw_data::car::mods::published().iter().map(|car| car.folder.clone()).collect()
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// The Spearman correlation of `values` with their own index: 1 for a steady rise.
fn rise(values: &[f32]) -> f32 {
    let rank = |values: &[f32]| -> Vec<f32> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        order.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
        let mut ranks = vec![0.0; values.len()];
        for (position, index) in order.into_iter().enumerate() {
            ranks[index] = position as f32;
        }
        ranks
    };
    if values.windows(2).all(|pair| pair[0] == pair[1]) {
        return 0.0;
    }
    let (a, b) = (rank(&(0..values.len()).map(|n| n as f32).collect::<Vec<_>>()), rank(values));
    let n = a.len() as f32;
    let (mean_a, mean_b) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cross: f32 = a.iter().zip(&b).map(|(x, y)| (x - mean_a) * (y - mean_b)).sum();
    let spread = (a.iter().map(|x| (x - mean_a).powi(2)).sum::<f32>()
        * b.iter().map(|y| (y - mean_b).powi(2)).sum::<f32>())
    .sqrt();
    if spread > 0.0 { cross / spread } else { 0.0 }
}

/// The checks every converted car must pass; the failures come back as lines.
fn check(car: &str, m: &Metrics) -> Vec<String> {
    let mut failures = Vec::new();
    let mut need = |ok: bool, what: String| {
        if !ok {
            failures.push(format!("{car}: {what}"));
        }
    };
    let steady: Vec<f32> = STEPS.iter().skip(1).map(|step| m.get(&steady_key(1.0, *step, "db"))).collect();
    let (low, high) = steady.iter().fold((f32::MAX, f32::MIN), |(low, high), db| (low.min(*db), high.max(*db)));
    need(low >= -25.0 && high <= -14.0, format!("steady loudness {low:.1} to {high:.1} dB, not -25 to -14"));
    need(high - low <= 9.0, format!("steady loudness spreads {:.1} dB over the revs, not at most 9", high - low));
    let idle = m.get(&steady_key(1.0, 0.0, "db"));
    need((-34.0..=-14.0).contains(&idle), format!("idle at {idle:.1} dB, not -34 to -14"));
    let part = m.get(&steady_key(0.3, 0.5, "db"));
    need((-30.0..=-14.0).contains(&part), format!("30 percent throttle at {part:.1} dB, not -30 to -14"));
    for name in ["sweep", "sweep_part", "coast"] {
        need(m.get(&format!("{name}.clip")) == 0.0, format!("{name} clips"));
        need(
            m.get(&format!("{name}.peak")) <= -3.0,
            format!("{name} peaks at {:.1} dB, not at most -3", m.get(&format!("{name}.peak"))),
        );
        need(m.get(&format!("{name}.gaps")) == 0.0, format!("{name} has holes"));
        need(
            m.get(&format!("{name}.step100ms")) <= 10.0,
            format!("{name} steps {:.1} dB in 100 ms", m.get(&format!("{name}.step100ms"))),
        );
        need(m.get(&format!("{name}.click")) <= 5.0, format!("{name} clicks ({:.1})", m.get(&format!("{name}.click"))));
    }
    let centroids: Vec<f32> = STEPS.iter().map(|step| m.get(&steady_key(1.0, *step, "centroid"))).collect();
    need(rise(&centroids) >= 0.0, format!("the sound gets duller with the revs ({:.2})", rise(&centroids)));
    need(
        m.get("sweep.centroid_last") <= 1800.0,
        format!("hisses at the limiter: centroid {:.0} Hz", m.get("sweep.centroid_last")),
    );
    need(
        m.get("start.sfx.peak") <= -8.0,
        format!("the shifts and pops peak at {:.1} dB, not at most -8", m.get("start.sfx.peak")),
    );
    need(
        m.get("start.sfx.loud") <= -24.0,
        format!("the shifts and pops are at {:.1} dB, not at most -24", m.get("start.sfx.loud")),
    );
    need(
        m.get("start.sfx_minus_engine_db") <= -4.0,
        format!("the effects are {:.1} dB from the engine", m.get("start.sfx_minus_engine_db")),
    );
    need(m.get("start.sfx.bursts_per_s") <= 0.7, format!("{:.2} bangs a second", m.get("start.sfx.bursts_per_s")));
    need(
        (-22.0..=-12.0).contains(&m.get("nitrous.engine.loud")),
        format!("nitrous engine at {:.1} dB", m.get("nitrous.engine.loud")),
    );
    need(m.get("start.all.peak") <= -0.5 && m.get("start.all.clip") == 0.0, "the whole drive peaks too high".into());
    failures
}

#[test]
#[ignore = "renders every converted car offline (NFSMW_GAME_DIR, NFSMW_MEASURE_MOD)"]
fn every_converted_car_is_as_loud_and_as_smooth_as_the_cars_of_the_game() {
    let (Some(game), Some(mod_dir)) = (std::env::var_os("NFSMW_GAME_DIR"), std::env::var_os("NFSMW_MEASURE_MOD"))
    else {
        return;
    };
    let rig = Rig::open(&PathBuf::from(game), Some(&PathBuf::from(mod_dir)));
    let reference = report::measure(&rig, "BMWM3GTR");
    let reference_loudness =
        median(&mut STEPS.iter().map(|step| reference.get(&steady_key(1.0, *step, "db"))).collect::<Vec<_>>());
    let (mut failures, mut medians, mut rises) = (Vec::new(), Vec::new(), Vec::new());
    let cars = converted();
    for car in &cars {
        let metrics = report::measure(&rig, car);
        failures.extend(check(car, &metrics));
        medians
            .push(median(&mut STEPS.iter().map(|step| metrics.get(&steady_key(1.0, *step, "db"))).collect::<Vec<_>>()));
        rises
            .push(rise(&STEPS.iter().map(|step| metrics.get(&steady_key(1.0, *step, "centroid"))).collect::<Vec<_>>()));
    }
    let (car_median, rise_median) = (median(&mut medians), median(&mut rises));
    println!(
        "{} cars: median loudness {car_median:.1} dB against {reference_loudness:.1} dB for the BMW M3 GTR; the centroid rises with the revs at {rise_median:.2}",
        cars.len()
    );
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    assert!((car_median - reference_loudness).abs() <= 3.0, "{car_median} against {reference_loudness}");
    assert!(rise_median >= 0.8, "the pitch follows the revs: {rise_median}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steady_rise_is_one_a_fall_minus_one_and_a_flat_line_zero() {
        assert!((rise(&[1.0, 2.0, 3.0, 4.0]) - 1.0).abs() < 1e-6);
        assert!((rise(&[4.0, 3.0, 2.0, 1.0]) + 1.0).abs() < 1e-6);
        assert_eq!(rise(&[2.0, 2.0, 2.0]), 0.0);
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
    }
}
