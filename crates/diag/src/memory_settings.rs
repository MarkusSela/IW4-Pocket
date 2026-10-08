//! Small, restart-only memory controls shared by loading, rendering and reports.
//! Environment is read once, after the iOS settings file and before loading.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

pub const FPV_CACHE_ENV: &str = "IW4L_FPV_RETAIN_MIB";
pub const SHADER_WORKERS_ENV: &str = "IW4L_SHADER_WORKERS";
pub const MOVE_IMAGES_ENV: &str = "IW4L_MOVE_IMAGES";
pub const RESIDENT_MAP_ENV: &str = "IW4L_RESIDENT_MAP";
pub const IOS_BC_ENV: &str = "IW4L_IOS_BC";

/// Device class, chosen once at startup from the memory iOS grants this process.
/// Defaults only: every `IW4L_*` switch (iw4l-env.txt) still wins over the tier.
/// Thresholds are first guesses from two phones (3070 MiB and 6141 MiB granted);
/// they get tuned with the device logs people send in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Under 4 GiB granted: everything that costs memory is off or minimal.
    Low,
    /// 4 to 5.5 GiB granted.
    Mid,
    /// 5.5 GiB or more granted, or not iOS (no meaningful per-app limit).
    High,
}

impl Tier {
    /// `granted_mib` is what iOS grants at launch (`os_proc_available_memory`);
    /// `None` when unknown (desktop, or the call failed), which counts as High
    /// off iOS and as Low on iOS (the safe side).
    pub fn pick(ios: bool, granted_mib: Option<u64>) -> Self {
        match (ios, granted_mib) {
            (false, _) => Tier::High,
            (true, None) => Tier::Low,
            (true, Some(m)) if m < 4096 => Tier::Low,
            (true, Some(m)) if m < 5632 => Tier::Mid,
            (true, Some(_)) => Tier::High,
        }
    }

    pub fn name(self) -> &'static str {
        match self { Tier::Low => "low", Tier::Mid => "mid", Tier::High => "high" }
    }

    /// Next-map FPV payload cache in MiB (0 = off). u64::MAX = unlimited (desktop).
    fn fpv_default(self, ios: bool) -> u64 {
        if !ios { return u64::MAX; }
        match self { Tier::Low => 0, Tier::Mid => 64, Tier::High => 256 }
    }
    /// Shader compile workers (0 = whole pool).
    fn shader_workers_default(self, ios: bool) -> u64 {
        if !ios { return 0; }
        match self { Tier::Low => 1, Tier::Mid | Tier::High => 2 }
    }
    /// Keep the walked match for an instant same-map reload. It pins every decoded
    /// image, so on iOS it is off for every tier: the only device run on a 6 GB phone
    /// (treuenten, issue #1) was killed with it on and completed with it off.
    fn resident_map_default(self, ios: bool) -> bool {
        !ios
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemorySettings {
    pub tier: Tier,
    /// Optional next-map FPV payload cache, not active weapon data or donor batches.
    pub fpv_retain_bytes: u64,
    /// Zero means use the existing load pool width (desktop default).
    pub shader_workers: usize,
    pub move_images: bool,
    /// Keep the walked match for an instant same-map reload. It shares every
    /// decoded image, so their texels cannot move to the GPU (off on iOS).
    pub resident_map: bool,
    /// Keep BC textures compressed on iOS when the GPU samples BC.
    pub ios_bc: bool,
}

fn flag(value: Option<&str>, default: bool) -> bool {
    match value.map(str::trim) {
        Some("0") => false,
        Some("1") => true,
        _ => default,
    }
}

fn bounded(value: Option<&str>, default: u64, min: u64, max: u64) -> u64 {
    value.and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|n| (min..=max).contains(n)).unwrap_or(default)
}

impl MemorySettings {
    fn parse(
        ios: bool,
        tier: Tier,
        fpv: Option<&str>,
        shaders: Option<&str>,
        moves: Option<&str>,
        resident: Option<&str>,
        bc: Option<&str>,
    ) -> Self {
        let default_fpv = match tier.fpv_default(ios) {
            u64::MAX => u64::MAX,
            mib => mib * 1024 * 1024,
        };
        let fpv_retain_bytes = fpv.and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|n| *n <= 1024).map(|n| n * 1024 * 1024).unwrap_or(default_fpv);
        Self {
            tier,
            fpv_retain_bytes,
            shader_workers: bounded(shaders, tier.shader_workers_default(ios), 1, 64) as usize,
            move_images: match moves.map(str::trim) {
                Some("0") => false,
                Some("1") => true,
                _ => true,
            },
            resident_map: flag(resident, tier.resident_map_default(ios)),
            ios_bc: flag(bc, true),
        }
    }

    pub fn effective_shader_workers(self, pool_width: usize) -> usize {
        let pool_width = pool_width.max(1);
        if self.shader_workers == 0 { pool_width } else { self.shader_workers.min(pool_width) }
    }

    pub fn report(self) -> String {
        let fpv = if self.fpv_retain_bytes == u64::MAX {
            "unlimited (desktop default)".to_owned()
        } else {
            (self.fpv_retain_bytes / (1024 * 1024)).to_string()
        };
        format!("device tier: {} | memory settings: {FPV_CACHE_ENV}={fpv} MiB; {SHADER_WORKERS_ENV}={} requested (0=pool); {MOVE_IMAGES_ENV}={}; {RESIDENT_MAP_ENV}={}; {IOS_BC_ENV}={}",
            self.tier.name(), self.shader_workers, u8::from(self.move_images), u8::from(self.resident_map),
            u8::from(self.ios_bc))
    }
}

/// Memory iOS grants this process at launch, in MiB; 0 = not recorded.
/// The launcher records it before the first `get()`. Kept here (not read from
/// `ios_env`) so this file compiles and tests on its own.
static GRANTED_MIB: AtomicU64 = AtomicU64::new(0);

pub fn record_granted_mib(mib: u64) {
    GRANTED_MIB.store(mib, Ordering::Relaxed);
}

pub fn get() -> &'static MemorySettings {
    static SETTINGS: OnceLock<MemorySettings> = OnceLock::new();
    SETTINGS.get_or_init(|| {
        let ios = cfg!(target_os = "ios");
        let granted_mib = Some(GRANTED_MIB.load(Ordering::Relaxed)).filter(|m| *m > 0);
        MemorySettings::parse(
            ios,
            Tier::pick(ios, granted_mib),
            std::env::var(FPV_CACHE_ENV).ok().as_deref(),
            std::env::var(SHADER_WORKERS_ENV).ok().as_deref(),
            std::env::var(MOVE_IMAGES_ENV).ok().as_deref(),
            std::env::var(RESIDENT_MAP_ENV).ok().as_deref(),
            std::env::var(IOS_BC_ENV).ok().as_deref(),
        )
    })
}

/// No overflow and no partial payloads. Zero really disables retention.
pub fn admits_bytes(held: u64, next: u64, limit: u64) -> bool {
    limit != 0 && next <= limit.saturating_sub(held) && held <= limit
}

/// Only these restart-time tuning switches may be set from the iOS text file.
/// Paths, external endpoints and arbitrary engine controls are deliberately excluded.
pub fn valid_file_setting(name: &str, value: &str) -> bool {
    match name {
        FPV_CACHE_ENV => value.parse::<u64>().is_ok_and(|n| n <= 1024),
        SHADER_WORKERS_ENV => value.parse::<u64>().is_ok_and(|n| (1..=64).contains(&n)),
        MOVE_IMAGES_ENV | RESIDENT_MAP_ENV | IOS_BC_ENV => matches!(value, "0" | "1"),
        "IW4L_IMAGE_DECODE_BUDGET_MIB" | "IW4L_CACHE_BUDGET_MIB" =>
            value.parse::<u64>().is_ok_and(|n| n <= 4096),
        "IW4L_SOUND" => matches!(value, "off" | "0" | "on" | "1"),
        // Metal keeps at most three drawables, so latency above 2 does nothing.
        "IW4L_FRAME_LATENCY" => matches!(value, "1" | "2"),
        // 0 keeps the display at 60 Hz instead of asking for 120 Hz in matches.
        "IW4L_PROMOTION" => matches!(value, "0" | "1"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_defaults_and_invalid_values() {
        let ios = MemorySettings::parse(true, Tier::Low, None, None, None, None, None);
        assert_eq!(ios.fpv_retain_bytes, 0);
        assert_eq!(ios.effective_shader_workers(4), 1);
        assert!(ios.move_images);
        assert!(!ios.resident_map);
        assert!(ios.ios_bc);
        let invalid = MemorySettings::parse(true, Tier::Low, Some("-1"), Some("0"), Some("bad"), Some("bad"), Some("bad"));
        assert_eq!(invalid.fpv_retain_bytes, 0);
        assert_eq!(invalid.shader_workers, 1);
        assert!(!invalid.resident_map);
        assert!(invalid.ios_bc);
        let desktop = MemorySettings::parse(false, Tier::High, None, None, None, None, None);
        assert_eq!(desktop.fpv_retain_bytes, u64::MAX);
        assert_eq!(desktop.effective_shader_workers(4), 4);
        assert!(desktop.resident_map);
    }
    #[test]
    fn ab_settings_are_bounded() {
        let s = MemorySettings::parse(true, Tier::Low, Some("64"), Some("64"), Some("0"), Some("1"), Some("0"));
        assert_eq!(s.fpv_retain_bytes, 64 * 1024 * 1024);
        assert_eq!(s.effective_shader_workers(4), 4);
        assert_eq!(s.effective_shader_workers(0), 1);
        assert!(!s.move_images);
        assert!(s.resident_map);
        assert!(!s.ios_bc);
        assert_eq!(MemorySettings::parse(true, Tier::Low, Some("18446744073709551615"), Some("65"), None, None, None).fpv_retain_bytes, 0);
    }
    #[test]
    fn tier_boundaries_follow_granted_memory() {
        assert_eq!(Tier::pick(true, Some(3070)), Tier::Low);
        assert_eq!(Tier::pick(true, Some(4095)), Tier::Low);
        assert_eq!(Tier::pick(true, Some(4096)), Tier::Mid);
        assert_eq!(Tier::pick(true, Some(5631)), Tier::Mid);
        assert_eq!(Tier::pick(true, Some(5632)), Tier::High);
        assert_eq!(Tier::pick(true, Some(6141)), Tier::High);
        assert_eq!(Tier::pick(true, None), Tier::Low);
        assert_eq!(Tier::pick(false, None), Tier::High);
    }
    #[test]
    fn tiers_set_defaults_and_env_still_wins() {
        let low = MemorySettings::parse(true, Tier::Low, None, None, None, None, None);
        assert_eq!((low.fpv_retain_bytes, low.effective_shader_workers(4), low.resident_map), (0, 1, false));
        let mid = MemorySettings::parse(true, Tier::Mid, None, None, None, None, None);
        assert_eq!((mid.fpv_retain_bytes, mid.effective_shader_workers(4), mid.resident_map), (64 << 20, 2, false));
        let high = MemorySettings::parse(true, Tier::High, None, None, None, None, None);
        assert_eq!((high.fpv_retain_bytes, high.effective_shader_workers(4), high.resident_map), (256 << 20, 2, false));
        let forced = MemorySettings::parse(true, Tier::Low, Some("128"), Some("3"), None, Some("1"), None);
        assert_eq!((forced.fpv_retain_bytes, forced.effective_shader_workers(4), forced.resident_map), (128 << 20, 3, true));
    }
    #[test]
    fn budget_boundaries() {
        assert!(!admits_bytes(0, 0, 0));
        assert!(admits_bytes(32, 32, 64));
        assert!(!admits_bytes(32, 33, 64));
        assert!(!admits_bytes(65, 0, 64));
        assert!(!admits_bytes(u64::MAX - 1, 2, u64::MAX));
    }
    #[test]
    fn file_allowlist_rejects_paths_unknowns_and_nuls() {
        assert!(valid_file_setting(FPV_CACHE_ENV, "0"));
        assert!(valid_file_setting(SHADER_WORKERS_ENV, "4"));
        assert!(valid_file_setting(RESIDENT_MAP_ENV, "1"));
        assert!(valid_file_setting(IOS_BC_ENV, "0"));
        assert!(!valid_file_setting(IOS_BC_ENV, "yes"));
        assert!(!valid_file_setting(SHADER_WORKERS_ENV, "0"));
        assert!(!valid_file_setting("IW4L_GAMES", "/tmp"));
        assert!(!valid_file_setting("IW4L_UNKNOWN", "1"));
        assert!(!valid_file_setting(FPV_CACHE_ENV, "1\0"));
        assert!(!valid_file_setting("IW4L_\0", "1"));
    }
}
