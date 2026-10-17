//! The storage formats of the intermediate images, chosen once per context.
//!
//! AMD's FSR 3 keeps most of its intermediate images in 16-bit floats. WebGPU core only lets a shader
//! write `R16Float` or `Rg16Float` when the device enables [`Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`]
//! and the adapter reports storage for that format, and it lets a shader read and write in place only
//! the 32-bit single-channel format. Each choice therefore falls back to its 32-bit twin when a condition
//! is missing. The depth-related images stay 32-bit in every case.

use wgpu::{Adapter, Device, Features, TextureFormat, TextureFormatFeatureFlags, TextureFormatFeatures, TextureUsages};

/// The usages every intermediate image is created with: sampled by `textureLoad`, written by a compute
/// shader and copied out for the debug views.
const IMAGE_USAGES: TextureUsages =
    TextureUsages::TEXTURE_BINDING.union(TextureUsages::STORAGE_BINDING).union(TextureUsages::COPY_SRC);

/// The formats of the intermediate images that a 16-bit choice can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Intermediates {
    /// Two channels: the dilated motion vectors and the shading change pyramid (AMD: `R16G16_FLOAT`).
    pub pair: TextureFormat,
    /// One channel, written by one pass: the luma, the shading change, the luma instability and the
    /// accumulation (AMD: `R16_FLOAT`).
    pub single: TextureFormat,
    /// The new locks, which are read and written in place (AMD: `R16_FLOAT`).
    pub lock: TextureFormat,
}

impl Intermediates {
    /// 32-bit floats everywhere: what a context made without the adapter uses.
    pub const FULL: Self =
        Self { pair: TextureFormat::Rg32Float, single: TextureFormat::R32Float, lock: TextureFormat::R32Float };

    /// The 16-bit formats where the device and the adapter can store them, 32-bit floats elsewhere.
    pub fn probe(device: &Device, adapter: &Adapter) -> Self {
        let enabled = device.features().contains(Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        Self::choose(enabled, |format| adapter.get_texture_format_features(format))
    }

    /// The choice, given whether the device enabled the adapter-specific format features and what the
    /// adapter supports for a format.
    pub(crate) fn choose(enabled: bool, features: impl Fn(TextureFormat) -> TextureFormatFeatures) -> Self {
        if !enabled {
            return Self::FULL;
        }
        let supports = |format: TextureFormat, access: TextureFormatFeatureFlags| {
            let found = features(format);
            found.allowed_usages.contains(IMAGE_USAGES) && found.flags.contains(access)
        };
        let write = TextureFormatFeatureFlags::STORAGE_WRITE_ONLY;
        let read_write = TextureFormatFeatureFlags::STORAGE_READ_WRITE;
        Self {
            pair: pick(supports(TextureFormat::Rg16Float, write), TextureFormat::Rg16Float, TextureFormat::Rg32Float),
            single: pick(supports(TextureFormat::R16Float, write), TextureFormat::R16Float, TextureFormat::R32Float),
            lock: pick(supports(TextureFormat::R16Float, read_write), TextureFormat::R16Float, TextureFormat::R32Float),
        }
    }
}

/// `half` when the adapter supports it, else `full`.
fn pick(supported: bool, half: TextureFormat, full: TextureFormat) -> TextureFormat {
    match supported {
        true => half,
        false => full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(allowed: TextureUsages, flags: TextureFormatFeatureFlags) -> TextureFormatFeatures {
        TextureFormatFeatures { allowed_usages: allowed, flags }
    }

    /// An adapter that stores every format with read, write and read-write access.
    fn full_support(_format: TextureFormat) -> TextureFormatFeatures {
        features(
            IMAGE_USAGES | TextureUsages::RENDER_ATTACHMENT,
            TextureFormatFeatureFlags::STORAGE_READ_ONLY
                | TextureFormatFeatureFlags::STORAGE_WRITE_ONLY
                | TextureFormatFeatureFlags::STORAGE_READ_WRITE,
        )
    }

    #[test]
    fn without_the_device_feature_everything_is_32_bit() {
        assert_eq!(Intermediates::choose(false, full_support), Intermediates::FULL);
    }

    #[test]
    fn with_full_adapter_support_the_reference_formats_are_used() {
        let expected = Intermediates {
            pair: TextureFormat::Rg16Float,
            single: TextureFormat::R16Float,
            lock: TextureFormat::R16Float,
        };
        assert_eq!(Intermediates::choose(true, full_support), expected);
    }

    #[test]
    fn a_format_without_storage_falls_back_to_32_bit() {
        let no_16_bit_storage = |format: TextureFormat| match format {
            TextureFormat::R16Float | TextureFormat::Rg16Float => {
                features(IMAGE_USAGES, TextureFormatFeatureFlags::empty())
            }
            _ => full_support(format),
        };
        assert_eq!(Intermediates::choose(true, no_16_bit_storage), Intermediates::FULL);
    }

    #[test]
    fn the_lock_needs_read_write_storage_on_its_own() {
        let write_only = |format: TextureFormat| match format {
            TextureFormat::R16Float => features(IMAGE_USAGES, TextureFormatFeatureFlags::STORAGE_WRITE_ONLY),
            _ => full_support(format),
        };
        let chosen = Intermediates::choose(true, write_only);
        assert_eq!(chosen.single, TextureFormat::R16Float);
        assert_eq!(chosen.lock, TextureFormat::R32Float);
    }

    #[test]
    fn storage_without_the_usage_flag_is_not_enough() {
        let no_usage = |format: TextureFormat| {
            let found = full_support(format);
            features(found.allowed_usages.difference(TextureUsages::STORAGE_BINDING), found.flags)
        };
        assert_eq!(Intermediates::choose(true, no_usage), Intermediates::FULL);
    }
}
