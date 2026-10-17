use crate::app::test_backend::NullBackend;
use crate::settings::test_caps;
use blackbox_gfx::RenderBackend;
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

#[test]
fn mip_residency_reduces_physical_dimensions_and_handles_small_bc_tails() {
    for compressed in [true, false] {
        let mut caps = test_caps::native();
        caps.compressed_bc = compressed;
        let mut backend = NullBackend::new(caps).with_texture_tracking();
        let texture = Texture {
            name: "fixture".into(),
            name_hash: 1,
            width: 16,
            height: 16,
            mip_levels: 5,
            format: PixelFormat::Dxt1,
            compression_type: 0x22,
            alpha_usage: AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            palette: vec![],
            data: vec![0; 128 + 32 + 8 + 8 + 8],
        };
        for discard in [0, 1, 2, 3, 4, 99] {
            let handle = blackbox_scene::upload_texture_predecoded_mips(&mut backend, &texture, None, discard).unwrap();
            let first = discard.min(4);
            let expected_mips = match compressed && first <= 2 {
                true => 5 - first,
                false => 1,
            };
            assert_eq!(backend.texture_sizes[&handle], [16 >> first, 16 >> first, expected_mips]);
            backend.destroy_texture(handle);
        }
        assert_eq!(backend.resources.lock().unwrap().textures, 0);
    }
}

#[test]
fn nonzero_mip_never_reuses_level_zero_decoded_pixels() {
    let texture = Texture {
        name: "fixture".into(),
        name_hash: 1,
        width: 2,
        height: 2,
        mip_levels: 2,
        format: PixelFormat::Argb8888,
        compression_type: 32,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: false,
        alpha_blend: 0,
        palette: vec![],
        data: [vec![0; 16], vec![1, 2, 3, 4]].concat(),
    };
    assert_eq!(blackbox_tpk::decode_mip_rgba8(&texture, 1).unwrap(), [3, 2, 1, 4]);
    assert!(blackbox_tpk::decode_mip_rgba8(&texture, 2).is_none());
    assert!(blackbox_tpk::decode_mip_rgba8(&texture, 32).is_none());
}
