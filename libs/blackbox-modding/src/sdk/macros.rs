//! `declare_mod!`: exports the two entry symbols for a mod type.

/// Export the entry symbols of a mod. Call it once per library, with the type that implements `Mod`:
///
/// ```ignore
/// blackbox_modding::sdk::declare_mod!(MyMod);
/// ```
///
/// It emits `bb_mod_info` (the info the loader reads first) and `bb_mod_create` (the handshake, then `load`). The
/// mod's `event` and `unload` are reached through the mod table the create call fills in. Every callback runs
/// inside the SDK's panic guard.
#[macro_export]
macro_rules! declare_mod {
    ($mod_ty:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn bb_mod_info() -> *const $crate::abi::BbModInfo {
            static INFO: $crate::abi::BbModInfo = $crate::abi::BbModInfo::for_sdk($crate::sdk::SDK_VERSION);
            &INFO
        }

        #[unsafe(no_mangle)]
        #[allow(unsafe_code)]
        pub unsafe extern "C" fn bb_mod_create(
            host: *const $crate::abi::BbHost,
            out: *mut $crate::abi::BbModApi,
        ) -> $crate::abi::BbStatus {
            // SAFETY: the loader passes a host table and an output slot that stay valid for this call
            // (ADR 0005 section 1). `ffi::create` checks both for null before it uses them.
            unsafe { $crate::sdk::ffi::create::<$mod_ty>(host, out) }
        }
    };
}
