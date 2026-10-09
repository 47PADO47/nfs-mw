//! `cargo xtask img-diff A.png B.png [--out DIFF.png]`: how far apart two screenshots are.
//!
//! For comparing renderers on the real game install, where the pictures cannot be committed: it reads two PNGs
//! from anywhere, prints the mean, 99th percentile and maximum absolute difference of the red, green and blue
//! samples (out of 255), and can write the difference, amplified 16 times, as a PNG.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

struct Rgba {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

fn read(path: &str) -> Result<Rgba, String> {
    let file = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("{path}: {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or_else(|| format!("{path}: too large"))?];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("{path}: {e}"))?;
    buf.truncate(info.buffer_size());
    let data = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => return Err(format!("{path}: unsupported colour type {other:?}")),
    };
    Ok(Rgba { width: info.width, height: info.height, data })
}

/// The statistics of the absolute differences of the colour samples.
#[derive(Debug, PartialEq)]
pub struct Diff {
    pub mean: f64,
    pub p99: u8,
    pub max: u8,
}

pub fn diff(a: &Rgba, b: &Rgba) -> Result<(Diff, Vec<u8>), String> {
    if (a.width, a.height) != (b.width, b.height) {
        return Err(format!("sizes differ: {}x{} and {}x{}", a.width, a.height, b.width, b.height));
    }
    let mut histogram = [0u64; 256];
    let mut amplified = Vec::with_capacity(a.data.len());
    let (mut sum, mut max) = (0u64, 0u8);
    for (pa, pb) in a.data.chunks(4).zip(b.data.chunks(4)) {
        for c in 0..3 {
            let d = pa[c].abs_diff(pb[c]);
            histogram[d as usize] += 1;
            sum += u64::from(d);
            max = max.max(d);
            amplified.push(d.saturating_mul(16));
        }
        amplified.push(255);
    }
    let samples = (a.data.len() / 4 * 3) as u64;
    let target = samples - samples / 100;
    let mut seen = 0;
    let p99 = (0..256usize)
        .find(|&d| {
            seen += histogram[d];
            seen >= target
        })
        .unwrap_or(255) as u8;
    Ok((Diff { mean: sum as f64 / samples.max(1) as f64, p99, max }, amplified))
}

fn write(path: &str, width: u32, height: u32, data: &[u8]) -> Result<(), String> {
    let file = File::create(Path::new(path)).map_err(|e| format!("{path}: {e}"))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().and_then(|mut w| w.write_image_data(data)).map_err(|e| format!("{path}: {e}"))
}

pub fn run(args: &[String]) -> Result<bool, String> {
    let files: Vec<&String> = args.iter().filter(|a| a.ends_with(".png")).collect();
    let [a, b] = files[..] else { return Err("usage: cargo xtask img-diff A.png B.png [--out DIFF.png]".into()) };
    let (image_a, image_b) = (read(a)?, read(b)?);
    let (stats, amplified) = diff(&image_a, &image_b)?;
    println!(
        "img-diff: mean {:.3} p99 {} max {} (out of 255), {}x{}",
        stats.mean, stats.p99, stats.max, image_a.width, image_a.height
    );
    if let Some(at) = args.iter().position(|a| a == "--out") {
        let out = args.get(at + 1).ok_or("--out needs a file")?;
        write(out, image_a.width, image_a.height, &amplified)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(value: u8) -> Rgba {
        Rgba { width: 10, height: 10, data: [value, value, value, 255].repeat(100) }
    }

    #[test]
    fn identical_images_have_no_difference() {
        let (stats, _) = diff(&flat(9), &flat(9)).unwrap();
        assert_eq!(stats, Diff { mean: 0.0, p99: 0, max: 0 });
    }

    #[test]
    fn the_statistics_are_per_colour_sample() {
        let mut b = flat(0);
        b.data[0] = 100;
        let (stats, amplified) = diff(&flat(0), &b).unwrap();
        assert_eq!(stats.max, 100);
        assert!((stats.mean - 100.0 / 300.0).abs() < 1e-9);
        assert_eq!(amplified[0], 255, "amplified and clamped");
        assert_eq!(stats.p99, 0, "one sample in 300 is under the 99th percentile");
    }

    #[test]
    fn different_sizes_are_an_error() {
        let small = Rgba { width: 1, height: 1, data: vec![0, 0, 0, 255] };
        assert!(diff(&flat(0), &small).is_err());
    }
}
