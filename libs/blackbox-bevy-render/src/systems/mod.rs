//! Render-world systems added to Bevy's own `Core3d` schedule: the world effect layer and the UI
//! layer, neither of which the material pipeline (`material/`) or the instance pool (`apply/`) draws.
//! Registered by [`crate::plugin::BlackboxBevyRenderPlugin`] directly on the render sub-app, since both
//! read [`crate::ops::Shared`] straight from the facade's bridge rather than through Bevy's extraction
//! (see `effects::extract_world` for the one piece, texture handles and fog, that does need extracting).

pub mod effects;
pub mod ui;
