use blackbox_hash::bstring_hash_bytes;

/// The key the game finds objects, scripts and resources by: the Black Box string hash of the upper-cased
/// name (`FEHashUpper`).
pub fn fe_hash_upper(name: &str) -> u32 {
    let upper: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    bstring_hash_bytes(&upper)
}

/// The run-time handle of a resource file name: the hash of the upper-cased base name without extension, so
/// `Outrun_Backing.tga` and `FONT_MW_BODY.ffn` give the keys of the texture and the font.
pub fn resource_handle(file_name: &str) -> u32 {
    let base = file_name.rsplit(['\\', '/']).next().unwrap_or(file_name);
    let stem = base.rsplit_once('.').map_or(base, |(s, _)| s);
    fe_hash_upper(stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_follow_the_stem() {
        assert_eq!(resource_handle("Outrun_Backing.tga"), fe_hash_upper("OUTRUN_BACKING"));
        assert_eq!(resource_handle("C:\\art\\FONT_MW_BODY.ffn"), 0x545570c6);
        assert_eq!(resource_handle("White16x16.bmp"), 0xe2a5c626);
    }

    #[test]
    fn object_names_are_case_insensitive() {
        assert_eq!(fe_hash_upper("SpeedometerGroup"), 0x941fff09);
        assert_eq!(fe_hash_upper("speedometergroup"), fe_hash_upper("SPEEDOMETERGROUP"));
    }
}
