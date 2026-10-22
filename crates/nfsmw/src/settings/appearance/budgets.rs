//! Shared overall and feature budgets for the game menu and graphics overlay.
use super::{Quality, Settings, VisualStyle};
use crate::settings::{CarShading, RendererKind, ShadowQuality};
use blackbox_gfx::reflection::{ReflectionSize, distance::ReflectionDistance};
use blackbox_gfx::{AoQuality, ReflectionQuality};

impl Settings {
    pub fn select_overall_quality(&mut self, quality: Quality) {
        if self.renderer == RendererKind::Blackbox {
            self.appearance.lighting.sun_shadows = true;
        }
        if self.renderer == RendererKind::Blackbox && self.appearance.style != VisualStyle::OriginalPc {
            self.ao = AoQuality::High;
        }
        self.select_graphics_quality(quality);
    }

    pub fn overall_quality(&self) -> Option<Quality> {
        let mut expected = *self;
        expected.select_overall_quality(self.appearance.quality);
        (*self == expected).then_some(self.appearance.quality)
    }

    pub fn select_ao_budget(&mut self, quality: AoQuality) {
        self.ao = quality;
        if self.renderer == RendererKind::Blackbox && quality == AoQuality::High {
            self.ao_method = crate::settings::AoMethod::XeGtao;
            self.ao_intensity = crate::settings::Percent(80);
            self.ao_depth_mips = true;
        }
    }

    pub fn select_reflection_budget(&mut self, quality: Option<Quality>) {
        let Some(quality) = quality else {
            self.reflections = ReflectionQuality::Off;
            return;
        };
        self.reflections = match quality {
            Quality::Low => ReflectionQuality::Low,
            Quality::Balanced => ReflectionQuality::Medium,
            Quality::High | Quality::VeryHigh => ReflectionQuality::High,
        };
        if self.renderer != RendererKind::Blackbox || self.car_shading != CarShading::Glossy {
            return;
        }
        let (range, size) = match quality {
            Quality::Low => (100, 128),
            Quality::Balanced => (200, 512),
            Quality::High => (300, 1024),
            Quality::VeryHigh => (500, 1024),
        };
        self.reflection_extra.distance = ReflectionDistance::Meters(range);
        self.reflection_cube_size = ReflectionSize::Fixed(size);
    }

    pub fn reflection_budget(&self) -> Option<Quality> {
        if self.reflections == ReflectionQuality::Off {
            return None;
        }
        for &quality in Quality::offered(self.renderer) {
            let mut expected = *self;
            expected.select_reflection_budget(Some(quality));
            if *self == expected {
                return Some(quality);
            }
        }
        None
    }

    pub fn select_shadow_budget(&mut self, quality: Option<Quality>) {
        let Some(quality) = quality else {
            self.shadows = ShadowQuality::Off;
            self.appearance.lighting.sun_shadows = false;
            self.appearance.lighting.contact_shadows = false;
            return;
        };
        let mut expected = *self;
        expected.appearance.lighting.sun_shadows = true;
        expected.select_graphics_quality(quality);
        self.copy_shadow_budget(&expected);
    }

    pub fn shadow_budget(&self) -> Option<Quality> {
        if self.shadows == ShadowQuality::Off {
            return None;
        }
        for &quality in Quality::offered(self.renderer) {
            let mut expected = *self;
            expected.select_shadow_budget(Some(quality));
            if *self == expected {
                return Some(quality);
            }
        }
        None
    }

    fn copy_shadow_budget(&mut self, source: &Self) {
        self.shadows = source.shadows;
        if self.renderer != RendererKind::Blackbox {
            return;
        }
        let target = &mut self.appearance.lighting;
        let light = source.appearance.lighting;
        target.sun_shadows = light.sun_shadows;
        target.shadow_resolution = light.shadow_resolution;
        target.shadow_distance = light.shadow_distance;
        target.contact_length = light.contact_length;
        target.soft_shadows = light.soft_shadows;
        target.contact_shadows = light.contact_shadows;
        target.near_shadow_focus = light.near_shadow_focus;
        target.wide_shadow_detail = light.wide_shadow_detail;
        target.shadow_detail_resolution = light.shadow_detail_resolution;
        target.far_shadows = light.far_shadows;
        target.dense_shadow_filter = light.dense_shadow_filter;
    }
}
