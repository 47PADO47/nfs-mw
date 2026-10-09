//! Opaque names for resources a backend owns.
//!
//! A backend hands these out from `create_*` calls and recognises them again in later calls. The
//! number inside means something only to the backend that made the handle, so callers treat handles
//! as opaque; [`from_raw`](TextureHandle::from_raw) and [`raw`](TextureHandle::raw) exist for backends.

macro_rules! raw_handle {
    ($(#[$meta:meta])* $name:ident($inner:ty)) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name($inner);

        impl $name {
            /// Wrap a backend's own number. For backends only.
            pub const fn from_raw(raw: $inner) -> Self {
                Self(raw)
            }

            /// The backend's own number. For backends only.
            pub const fn raw(self) -> $inner {
                self.0
            }
        }
    };
}

raw_handle!(
    /// A texture uploaded with `create_texture`.
    TextureHandle(usize)
);

raw_handle!(
    /// A mesh uploaded with `create_mesh`.
    MeshHandle(usize)
);

raw_handle!(
    /// A glossy material registered with `create_glossy_material`; use it as
    /// [`Shading::Glossy`](crate::Shading::Glossy).
    GlossyMaterialHandle(usize)
);

impl GlossyMaterialHandle {
    /// Stands for "any glossy material" where only the pipeline matters.
    pub const ANY: Self = Self(usize::MAX);
}

/// Names a UI texture. Unlike the other handles the *caller* picks the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UiTextureId(pub u64);

impl UiTextureId {
    /// Same as constructing it directly; here so every handle has the same pair of methods.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

raw_handle!(
    /// A screenshot or capture that was requested and can be polled for.
    CaptureId(u64)
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_numbers_round_trip() {
        assert_eq!(TextureHandle::from_raw(7).raw(), 7);
        assert_eq!(MeshHandle::from_raw(0).raw(), 0);
        assert_eq!(GlossyMaterialHandle::from_raw(3).raw(), 3);
        assert_eq!(UiTextureId::from_raw(u64::MAX).raw(), u64::MAX);
        assert_eq!(CaptureId::from_raw(42).raw(), 42);
    }

    #[test]
    fn handles_compare_and_order_by_number() {
        assert_eq!(MeshHandle::from_raw(1), MeshHandle::from_raw(1));
        assert!(MeshHandle::from_raw(1) < MeshHandle::from_raw(2));
        assert_ne!(GlossyMaterialHandle::from_raw(1), GlossyMaterialHandle::ANY);
        assert_eq!(GlossyMaterialHandle::ANY.raw(), usize::MAX);
    }
}
