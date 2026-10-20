//! Normal / detailed diagnostics, and the one classifier that decides which log lines
//! are high-frequency.
//!
//! `IW4L_DIAG=normal|detailed` (iw4l-env.txt, IW4 Pocket options > Diagnostics) is read
//! once at startup, like the other env settings. Normal is the default.
//!
//! * Normal drops the high-frequency lines (bot decisions, per-frame render timings,
//!   static-model bucket counts, per-zone sound censuses). Everything else stays: start
//!   and effective config, load phases, bot add/refusal, class changes, memory pressure,
//!   lifecycle, warnings and errors, the periodic match summary.
//! * Detailed lets every line through (lines that already rate-limit themselves keep
//!   doing so).
//!
//! Warnings and errors are never dropped. Dropped lines are counted ([`suppressed`]), so
//! the aggregate is always available even when the lines are not.
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::Level;

/// The `iw4l-env.txt` name of the setting.
pub const DIAG_ENV: &str = "IW4L_DIAG";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiagDetail {
    #[default]
    Normal,
    Detailed,
}

impl DiagDetail {
    /// `detailed` selects Detailed; anything else (unset, `normal`, junk) is Normal.
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("detailed") => Self::Detailed,
            _ => Self::Normal,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Detailed => "detailed",
        }
    }
}

/// What kind of line a message is. This is the only place that knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventClass {
    /// Always written.
    Essential,
    /// Repeats every frame, tick or item: written in Detailed only.
    HighFrequency,
    /// Buries the useful lines (one per material, per model): kept out of the iOS boot
    /// log in every mode.
    Noise,
}

/// Prefixes of lines that repeat all the time. One bot decision per switch, one render
/// timing line every few frames, one bucket census per change, one line per sound zone.
const HIGH_FREQUENCY: &[&str] = &[
    "bots: id=",
    "smodel buckets:",
    "render frame diag:",
    "sound source:",
];

const NOISE: &[&str] = &[
    "menu catalog: material ",
    "menu catalog: UI image ",
    "hud: namespace tree ",
    "destructible death ",
    "world report: destructible death ",
    "static model mesh: ",
];

/// The single classifier. Cheap: a few prefix tests, no allocation.
pub fn classify(message: &str) -> EventClass {
    if NOISE.iter().any(|p| message.starts_with(p)) {
        EventClass::Noise
    } else if HIGH_FREQUENCY.iter().any(|p| message.starts_with(p)) {
        EventClass::HighFrequency
    } else {
        EventClass::Essential
    }
}

/// Whether a line is written at this detail level. Warnings and errors always are.
pub fn admits_at(detail: DiagDetail, lvl: Level, message: &str) -> bool {
    lvl <= Level::Warn
        || detail == DiagDetail::Detailed
        || classify(message) != EventClass::HighFrequency
}

/// The first bytes of a line being formatted. Writing stops (with an error that callers
/// ignore) once it is full, so the rest of the line is never rendered.
#[derive(Default)]
pub struct Head {
    buf: [u8; 32],
    len: usize,
}

impl Head {
    pub fn as_str(&self) -> &str {
        let bytes = &self.buf[..self.len];
        match std::str::from_utf8(bytes) {
            Ok(text) => text,
            // A multi-byte character cut by the limit: keep the complete part.
            Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or(""),
        }
    }
}

impl std::fmt::Write for Head {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let room = self.buf.len() - self.len;
        let take = text.len().min(room);
        self.buf[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
        self.len += take;
        if take < text.len() { Err(std::fmt::Error) } else { Ok(()) }
    }
}

const UNREAD: u8 = 2;
static DETAIL: AtomicU8 = AtomicU8::new(UNREAD);
static SUPPRESSED: AtomicU64 = AtomicU64::new(0);

/// The level for this run. Read from the environment on first use.
pub fn detail() -> DiagDetail {
    match DETAIL.load(Ordering::Relaxed) {
        0 => DiagDetail::Normal,
        1 => DiagDetail::Detailed,
        _ => reload_from_env(),
    }
}

/// Reads `IW4L_DIAG` again. The iOS env file loader calls this after it set the
/// variables, so a log line written earlier cannot pin the default.
pub fn reload_from_env() -> DiagDetail {
    let detail = DiagDetail::parse(std::env::var(DIAG_ENV).ok().as_deref());
    DETAIL.store(u8::from(detail == DiagDetail::Detailed), Ordering::Relaxed);
    detail
}

pub fn detailed() -> bool {
    detail() == DiagDetail::Detailed
}

/// Lines dropped by the classifier since start (cumulative).
pub fn suppressed() -> u64 {
    SUPPRESSED.load(Ordering::Relaxed)
}

pub fn note_suppressed() {
    SUPPRESSED.fetch_add(1, Ordering::Relaxed);
}

/// The "effective config" line for the start of the log.
pub fn config_line() -> String {
    format!("diagnostics: level={} ({DIAG_ENV}; normal drops per-frame/per-tick detail, warnings and errors always stay)", detail().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_normal() {
        assert_eq!(DiagDetail::parse(None), DiagDetail::Normal);
        assert_eq!(DiagDetail::parse(Some("normal")), DiagDetail::Normal);
        assert_eq!(DiagDetail::parse(Some("loud")), DiagDetail::Normal);
        assert_eq!(DiagDetail::parse(Some(" detailed ")), DiagDetail::Detailed);
    }

    #[test]
    fn classifier_splits_the_categories() {
        for hot in [
            "bots: id=3 tick=120 task=Roam",
            "smodel buckets: rigid=1 skinned=2 cached=3 unread=0 consume=1",
            "render frame diag: extract_ms=1.0",
            "sound source: `x.ff` captured in the common walk",
        ] {
            assert_eq!(classify(hot), EventClass::HighFrequency, "{hot}");
        }
        for kept in [
            "bots: add 3 (dummy=false) → clients [1, 2]",
            "bots: roster full — add 3 gave 1",
            "bots: window=64 bots=3 attempted[] denied[]",
            "bots: roster requested=3 added=3",
            "loading: map=mp_rust mode=dm",
            "frame pacing: pace=30 snap=on n=128",
            "match summary: t=30 s",
            "session summary: peak footprint 1 MiB",
            "lifecycle: process_exit pid=1",
        ] {
            assert_eq!(classify(kept), EventClass::Essential, "{kept}");
        }
        assert_eq!(classify("static model mesh: foo (2 surfaces)"), EventClass::Noise);
        assert_eq!(classify("static models: 3356 authored slots"), EventClass::Essential);
    }

    #[test]
    fn normal_drops_only_hot_info_lines() {
        let hot = "bots: id=1 tick=2";
        assert!(!admits_at(DiagDetail::Normal, Level::Info, hot));
        assert!(!admits_at(DiagDetail::Normal, Level::Debug, hot));
        assert!(admits_at(DiagDetail::Detailed, Level::Info, hot));
        // Warnings and errors always pass, whatever their text.
        assert!(admits_at(DiagDetail::Normal, Level::Warn, "render frame diag: RenderApp missing"));
        assert!(admits_at(DiagDetail::Normal, Level::Error, hot));
        assert!(admits_at(DiagDetail::Normal, Level::Info, "bots: roster requested=1"));
    }

    #[test]
    fn head_stops_after_the_prefix() {
        let mut head = Head::default();
        let r = std::fmt::write(&mut head, format_args!("bots: id={} tick={} {}", 3, 120, "x".repeat(200)));
        assert!(r.is_err());
        assert!(head.as_str().starts_with("bots: id=3 tick=120"));
        assert_eq!(classify(head.as_str()), EventClass::HighFrequency);
        let mut cut = Head::default();
        let _ = std::fmt::write(&mut cut, format_args!("{}é", "a".repeat(31)));
        assert_eq!(cut.as_str().len(), 31);
    }

    #[test]
    fn config_line_names_the_level() {
        assert!(config_line().starts_with("diagnostics: level="));
    }
}
