//! Private installed-patch evidence; no asset bytes or reference PCM are fixtures.
use blackbox_fxedit::{Patch, Processor};

const PATCHES: [&str; 15] = [
    "Alley.fx",
    "Car_Show_Bass.fx",
    "Car_Show_Bass2.fx",
    "Car_Show_Sml.fx",
    "City_Dense.fx",
    "City_Dense_Left.fx",
    "City_Dense_Right.fx",
    "City_Open.fx",
    "Garage.fx",
    "Garage_Sml.fx",
    "Hills.fx",
    "Hills_Close.fx",
    "Simple_Tunnel.fx",
    "Simple_Tunnel_Sml.fx",
    "Simple_Tunnel_w_EQ.fx",
];

fn signal() -> Vec<f32> {
    (0..12288)
        .map(|i| match i {
            0 => 0.5,
            1..2048 => ((i * 37 % 257) as f32 - 128.0) / 4096.0,
            _ => 0.0,
        })
        .collect()
}

fn render(patch: &Patch, rate: u32, block: usize, input: &[f32]) -> Vec<f32> {
    let mut processor = Processor::new(patch, rate, 2).unwrap();
    let mut output = Vec::with_capacity(input.len() * 2);
    let mut left = vec![0.0; block];
    let mut right = vec![0.0; block];
    for samples in input.chunks_exact(block) {
        processor.process_mono(samples, &mut left, &mut right).unwrap();
        output.extend(left.iter().zip(&right).flat_map(|(&l, &r)| [l, r]));
    }
    processor.reset();
    let mut reset = vec![0.0; 256];
    let mut reset_right = vec![0.0; 256];
    processor.process_mono(&[0.0; 256], &mut reset, &mut reset_right).unwrap();
    assert!(reset.iter().chain(&reset_right).all(|v| v.abs() < 1e-15));
    output
}

#[test]
#[ignore = "needs installed FXEDIT patches; set NFSMW_GAME_DIR"]
fn installed_fxedit_all_patches_rates_blocks_and_reset() {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("FXEDIT tests require NFSMW_GAME_DIR"),
    ))
    .unwrap();
    let evidence = std::env::var_os("NFSMW_FXEDIT_EVIDENCE_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &evidence {
        std::fs::create_dir_all(path).unwrap();
    }
    let input = signal();
    for name in PATCHES {
        let patch = Patch::parse(&dir.read(&format!("SOUND/FXEDIT/{name}")).unwrap()).unwrap();
        for rate in [44100, 48000] {
            let reference = render(&patch, rate, 256, &input);
            assert!(reference.iter().any(|v| v.abs() > 0.0001), "{name} {rate}: silent graph");
            for block in [16, 64] {
                let candidate = render(&patch, rate, block, &input);
                let maximum = reference.iter().zip(candidate).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
                assert!(maximum < 1e-5, "{name} {rate} block{block}: {maximum}");
            }
            if let Some(path) = &evidence {
                let bytes: Vec<u8> = reference.iter().flat_map(|s| s.to_le_bytes()).collect();
                std::fs::write(path.join(format!("{name}-{rate}.f32")), bytes).unwrap();
            }
        }
    }
}
