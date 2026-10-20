//! `IW4L_SELECTIVE_WEAPONS=auto|on|off`.

use diag::memory_settings::Tier;

pub const ENV_VAR: &str = "IW4L_SELECTIVE_WEAPONS";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectiveMode {
    /// Active on the low memory tier only.
    #[default]
    Auto,
    On,
    Off,
}

/// Parse the env value. `None`/empty => Auto; invalid => Auto plus a warning.
pub fn parse(value: Option<&str>) -> SelectiveMode {
    match value.map(|v| v.trim().to_ascii_lowercase()) {
        None => SelectiveMode::Auto,
        Some(v) => match v.as_str() {
            "" | "auto" => SelectiveMode::Auto,
            "on" | "1" | "true" => SelectiveMode::On,
            "off" | "0" | "false" => SelectiveMode::Off,
            other => {
                diag::warn!(
                    World,
                    "weapons: invalid {ENV_VAR}={other:?} (expected auto|on|off); using auto"
                );
                SelectiveMode::Auto
            }
        },
    }
}

pub fn is_active(mode: SelectiveMode, tier_low: bool) -> bool {
    match mode {
        SelectiveMode::Auto => tier_low,
        SelectiveMode::On => true,
        SelectiveMode::Off => false,
    }
}

pub fn selective_mode_from_env() -> SelectiveMode {
    parse(std::env::var(ENV_VAR).ok().as_deref())
}

/// Reads the env switch and the process-wide device tier.
pub fn selective_active_from_env() -> bool {
    let low = diag::memory_settings::get().tier == Tier::Low;
    is_active(selective_mode_from_env(), low)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_values() {
        assert_eq!(parse(None), SelectiveMode::Auto);
        assert_eq!(parse(Some("auto")), SelectiveMode::Auto);
        assert_eq!(parse(Some("ON")), SelectiveMode::On);
        assert_eq!(parse(Some("off")), SelectiveMode::Off);
        assert_eq!(parse(Some("garbage")), SelectiveMode::Auto);
    }

    #[test]
    fn activation() {
        assert!(is_active(SelectiveMode::Auto, true));
        assert!(!is_active(SelectiveMode::Auto, false));
        assert!(is_active(SelectiveMode::On, false));
        assert!(!is_active(SelectiveMode::Off, true));
    }
}
