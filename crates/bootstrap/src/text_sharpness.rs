//! "Nitidezza testo/UI": separates the resolution of the 3D scene from the
//! resolution of the window (UI, HUD, text).
//!
//! `IW4L_TEXT_SHARPNESS=auto|off|native|high|medium` (default `auto`; unknown
//! values warn and fall back to `auto`). Menu text (IT): "Nitidezza testo/UI:
//! Auto/Nativa/Alta/Media".
//!
//! Given the 3D render scale `s` (`IW4L_RENDER_SCALE`), the UI scale `u` is:
//! - Native: 1.0 (window at the screen's own resolution);
//! - High:   max(s, 0.75);
//! - Medium: max(s, 0.5);
//! - Auto:   unchanged (u = s) in this build; the separation is opt-in.
//!
//! The scene scale relative to the UI surface is `s / u` (<= 1.0).
#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

pub(crate) const TEXT_SHARPNESS_ENV: &str = "IW4L_TEXT_SHARPNESS";

/// Smallest side, in pixels, of either surface.
const MIN_SIDE: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum SharpnessMode {
    #[default]
    Auto,
    /// Escape hatch: today's behaviour (whole window at the 3D scale).
    Off,
    Native,
    High,
    Medium,
}

impl SharpnessMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Some(Self::Auto),
            "off" | "spento" | "disattivata" => Some(Self::Off),
            "native" | "nativa" => Some(Self::Native),
            "high" | "alta" => Some(Self::High),
            "medium" | "media" => Some(Self::Medium),
            _ => None,
        }
    }

    pub(crate) fn from_env() -> Self {
        match std::env::var(TEXT_SHARPNESS_ENV) {
            Err(_) => Self::Auto,
            Ok(value) => Self::parse(&value).unwrap_or_else(|| {
                diag::warn!(
                    Launch,
                    "text sharpness: {TEXT_SHARPNESS_ENV}={value} is not auto|off|native|high|medium, using auto"
                );
                Self::Auto
            }),
        }
    }
}

/// Shared with the consumers that must agree on which size is 3D and which
/// is UI. `ui_*` is the window / swapchain / HUD size; `scene_*` is the size
/// the 3D scene is drawn at (never larger than the UI size).
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextSharpness {
    pub mode: SharpnessMode,
    pub ui_scale: f64,
    pub scene_scale: f64,
    pub ui_size: (u32, u32),
    pub scene_size: (u32, u32),
}

/// Rounds to the nearest even number, at least `MIN_SIDE`, at most `native`
/// (rounded down to even).
fn side(native: u32, scale: f64) -> u32 {
    let cap = (native & !1).max(MIN_SIDE);
    let even = ((f64::from(native) * scale / 2.0).round() as u32).saturating_mul(2);
    even.clamp(MIN_SIDE, cap)
}

/// The 3D / UI separation is wired (render_gpu `scene_split`: scene drawn in the
/// top-left scene-size area, then linearly stretched over the whole window
/// before post-fx and the HUD). It applies ONLY to an explicit Native / High /
/// Medium mode with a 3D scale below `SEPARATION_BELOW`. Auto (default) and Off
/// keep today's behaviour: the whole window at the 3D scale.
pub(crate) const SEPARATION_WIRED: bool = true;

/// Scene below this 3D scale is the only case the separation applies to.
const SEPARATION_BELOW: f64 = 0.6;

/// UI scale for a mode and a 3D scale (both clamped to 0.2..=1.0).
fn ui_scale_for(mode: SharpnessMode, scale3d: f64, wired: bool) -> f64 {
    let s = if !scale3d.is_finite() {
        1.0
    } else if matches!(mode, SharpnessMode::Auto | SharpnessMode::Off) {
        scale3d
    } else {
        scale3d.clamp(0.2, 1.0)
    };
    if !wired || s >= SEPARATION_BELOW {
        return s;
    }
    match mode {
        SharpnessMode::Off | SharpnessMode::Auto => s,
        SharpnessMode::Native => 1.0,
        SharpnessMode::High => s.max(0.75),
        SharpnessMode::Medium => s.max(0.5),
    }
}

/// Pure selection of the two sizes from the native pixel size.
pub(crate) fn select(mode: SharpnessMode, native: (u32, u32), scale3d: f64) -> TextSharpness {
    select_with(mode, native, scale3d, SEPARATION_WIRED)
}

fn select_with(mode: SharpnessMode, native: (u32, u32), scale3d: f64, wired: bool) -> TextSharpness {
    // Auto/Off: exactly today's value (no clamp); callers already reject <0.2.
    let passthrough = matches!(mode, SharpnessMode::Auto | SharpnessMode::Off);
    let s = if !scale3d.is_finite() {
        1.0
    } else if passthrough {
        scale3d
    } else {
        scale3d.clamp(0.2, 1.0)
    };
    let u = ui_scale_for(mode, s, wired);
    let ui_size = (side(native.0, u), side(native.1, u));
    let scene_size = (
        side(native.0, s).min(ui_size.0),
        side(native.1, s).min(ui_size.1),
    );
    TextSharpness {
        mode,
        ui_scale: u,
        scene_scale: s / u,
        ui_size,
        scene_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: (u32, u32) = (2868, 1320);

    #[test]
    fn unwired_or_off_is_todays_behaviour() {
        for m in [SharpnessMode::Auto, SharpnessMode::Off, SharpnessMode::Native, SharpnessMode::High, SharpnessMode::Medium] {
            for s in [0.2, 0.5, 0.6, 0.9, 1.0] {
                let r = select(m, N, s);
                if matches!(m, SharpnessMode::Off | SharpnessMode::Auto) || !SEPARATION_WIRED || s >= 0.6 {
                    assert_eq!(r.ui_size, r.scene_size, "{m:?} {s}");
                    assert!((r.scene_scale - 1.0).abs() < 1e-9);
                }
            }
        }
        assert_eq!(SharpnessMode::parse("off"), Some(SharpnessMode::Off));
    }

    #[test]
    fn auto_and_off_keep_today_behaviour() {
        for m in [SharpnessMode::Auto, SharpnessMode::Off] {
            for s in [0.2, 0.3, 0.5, 0.6, 1.0] {
                let r = select_with(m, N, s, true);
                assert_eq!(r.ui_size, r.scene_size, "{m:?} {s}");
            }
        }
    }

    #[test]
    fn explicit_modes_from_06_unchanged() {
        for m in [SharpnessMode::Native, SharpnessMode::High, SharpnessMode::Medium] {
            let r = select_with(m, N, 0.6, true);
            assert_eq!(r.ui_size, r.scene_size);
            assert_eq!(select_with(m, N, 1.0, true).ui_size, N);
        }
    }

    #[test]
    fn native_below_06_separates() {
        for s in [0.2, 0.3, 0.5] {
            let r = select_with(SharpnessMode::Native, N, s, true);
            assert_eq!(r.ui_size, N, "scale {s}");
            assert!(r.scene_size.0 < N.0);
        }
    }

    #[test]
    fn scene_sizes_even_and_scaled() {
        assert_eq!(select_with(SharpnessMode::Native, N, 0.2, true).scene_size, (574, 264));
        assert_eq!(select_with(SharpnessMode::Native, N, 0.3, true).scene_size, (860, 396));
        assert_eq!(select_with(SharpnessMode::Native, N, 0.5, true).scene_size, (1434, 660));
        assert_eq!(select_with(SharpnessMode::Native, N, 1.0, true).scene_size, N);
    }

    #[test]
    fn high_and_medium_floors() {
        assert_eq!(select_with(SharpnessMode::High, N, 0.3, true).ui_size, (2152, 990));
        assert_eq!(select_with(SharpnessMode::Medium, N, 0.3, true).ui_size, (1434, 660));
        assert_eq!(select_with(SharpnessMode::High, N, 0.9, true).ui_scale, 0.9);
        assert_eq!(select_with(SharpnessMode::Medium, N, 0.4, true).scene_size, (1148, 528));
    }

    #[test]
    fn never_exceeds_native_and_minimums() {
        for m in [SharpnessMode::Auto, SharpnessMode::Native, SharpnessMode::High, SharpnessMode::Medium] {
            for s in [0.0, 0.2, 0.3, 0.5, 1.0, 5.0, f64::NAN] {
                let r = select_with(m, (1281, 721), s, true);
                assert!(r.ui_size.0 <= 1280 && r.ui_size.1 <= 720);
                assert!(r.scene_size.0 <= r.ui_size.0 && r.scene_size.1 <= r.ui_size.1);
                assert!(r.scene_size.0 % 2 == 0 && r.ui_size.1 % 2 == 0);
                assert!(r.scene_size.0 >= MIN_SIDE && r.scene_size.1 >= MIN_SIDE);
            }
        }
        assert_eq!(select_with(SharpnessMode::Native, (1, 1), 0.2, true).scene_size, (2, 2));
    }

    #[test]
    fn parse_values() {
        assert_eq!(SharpnessMode::parse("Nativa"), Some(SharpnessMode::Native));
        assert_eq!(SharpnessMode::parse("HIGH"), Some(SharpnessMode::High));
        assert_eq!(SharpnessMode::parse("medium"), Some(SharpnessMode::Medium));
        assert_eq!(SharpnessMode::parse("auto"), Some(SharpnessMode::Auto));
        assert_eq!(SharpnessMode::parse("x"), None);
    }
}
