use super::*;

#[test]
fn original_voice_low_pass_vectors_cover_retarget_zero_bypass_and_reentry() {
    for (rate, first, retarget) in [
        (
            44100,
            [
                f32::from_bits(0x3cea0ea1),
                f32::from_bits(0x3ce35eab),
                f32::from_bits(0x3cdcdf9f),
                f32::from_bits(0x3cd69017),
                f32::from_bits(0x3cd06eb7),
                f32::from_bits(0x3cca7a2e),
                f32::from_bits(0x3cc4b134),
                f32::from_bits(0x3cbf128a),
                f32::from_bits(0x3cb99cfc),
                f32::from_bits(0x3cb44f5b),
                f32::from_bits(0x3caf2884),
                f32::from_bits(0x3caa275c),
                f32::from_bits(0x3ca54ace),
                f32::from_bits(0x3ca091d0),
                f32::from_bits(0x3c9bfb5c),
                f32::from_bits(0x3c978677),
            ],
            [
                f32::from_bits(0x3c8b27e5),
                f32::from_bits(0x3c7f97a5),
                f32::from_bits(0x3c6aba49),
                f32::from_bits(0x3c5790f4),
                f32::from_bits(0x3c45f80f),
                f32::from_bits(0x3c35cee9),
                f32::from_bits(0x3c26f77d),
                f32::from_bits(0x3c195639),
                f32::from_bits(0x3c0cd1cb),
                f32::from_bits(0x3c0152f4),
                f32::from_bits(0x3bed88b0),
                f32::from_bits(0x3bda24b7),
                f32::from_bits(0x3bc855f6),
                f32::from_bits(0x3bb7fb5a),
                f32::from_bits(0x3ba8f681),
                f32::from_bits(0x3b9b2b86),
            ],
        ),
        (
            48000,
            [
                f32::from_bits(0x3cd70a3d),
                f32::from_bits(0x3cd1652b),
                f32::from_bits(0x3ccbe609),
                f32::from_bits(0x3cc68bd6),
                f32::from_bits(0x3cc1559b),
                f32::from_bits(0x3cbc4265),
                f32::from_bits(0x3cb7514b),
                f32::from_bits(0x3cb28166),
                f32::from_bits(0x3cadd1d8),
                f32::from_bits(0x3ca941c6),
                f32::from_bits(0x3ca4d05e),
                f32::from_bits(0x3ca07cd1),
                f32::from_bits(0x3c9c4657),
                f32::from_bits(0x3c982c2c),
                f32::from_bits(0x3c942d93),
                f32::from_bits(0x3c9049d1),
            ],
            [
                f32::from_bits(0x3c85777b),
                f32::from_bits(0x3c76e9d7),
                f32::from_bits(0x3c64651a),
                f32::from_bits(0x3c5343ec),
                f32::from_bits(0x3c436ba1),
                f32::from_bits(0x3c34c38e),
                f32::from_bits(0x3c2734e4),
                f32::from_bits(0x3c1aaa86),
                f32::from_bits(0x3c0f10ef),
                f32::from_bits(0x3c045611),
                f32::from_bits(0x3bf4d26c),
                f32::from_bits(0x3be275d7),
                f32::from_bits(0x3bd179ce),
                f32::from_bits(0x3bc1c3de),
                f32::from_bits(0x3bb33b94),
                f32::from_bits(0x3ba5ca50),
            ],
        ),
    ] {
        let mut filter = LowPass::default();
        let mut impulse = [0.0; 16];
        impulse[0] = 1.0;
        filter.set_cutoff(rate, 630);
        filter.process(impulse.iter_mut());
        assert_eq!(impulse, first);
        let mut release = [0.0; 16];
        filter.set_cutoff(rate, 1800);
        filter.process(release.iter_mut());
        assert_eq!(release, retarget);
        let mut held = [0.0; 4];
        filter.set_cutoff(rate, 0);
        filter.process(held.iter_mut());
        assert_eq!(held, [retarget[15]; 4]);
        filter.set_cutoff(rate, 65535);
        let mut open = [0.5; 4];
        filter.process(open.iter_mut());
        assert_eq!(open, [0.5; 4]);
        filter.set_cutoff(rate, 630);
        let mut reentered = [0.0; 16];
        reentered[0] = 1.0;
        filter.process(reentered.iter_mut());
        assert_eq!(reentered, first);
        filter.set_cutoff(rate, rate / 2);
        let mut nyquist = [0.25; 4];
        filter.process(nyquist.iter_mut());
        assert_eq!(nyquist, [0.25; 4]);
    }
}
