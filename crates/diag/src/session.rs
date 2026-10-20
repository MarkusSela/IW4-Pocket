//! What one run looked like, kept for the last lines of the boot log and for the
//! next start: peak memory, the least headroom iOS left, and why the run ended.
//!
//! The memory logger feeds [`note_sample`] four times a second and prints
//! [`summary_line`] every few seconds, so a run that iOS kills silently still ends
//! with a recent summary. [`set_exit_reason`] records why a run that ends on its
//! own did so (the render-error handler sets it).
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static PEAK_FOOTPRINT_MIB: AtomicU64 = AtomicU64::new(0);
static MIN_AVAILABLE_MIB: AtomicU64 = AtomicU64::new(u64::MAX);
static EXIT_REASON: Mutex<String> = Mutex::new(String::new());
static SHADOWS_FORCED_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Dynamic shadows are held off whatever the game's Shadows option says (the phone
/// cannot draw them, or IW4 Pocket options turned them off). Set once at startup.
pub fn force_shadows_off(off: bool) {
    SHADOWS_FORCED_OFF.store(off, Ordering::Relaxed);
}

pub fn shadows_forced_off() -> bool {
    SHADOWS_FORCED_OFF.load(Ordering::Relaxed)
}

/// Whether dynamic shadows are held off, from the IW4 Pocket options value
/// (`IW4L_SHADOWS`: `0` off, `1` on, unset = Auto) and what the GPU can do.
/// Auto turns them off only where the shadow maps cannot be bound (no 32-bit float
/// filtering): the maps would otherwise be drawn for nothing.
pub fn shadows_decision(option: Option<&str>, float32_filtering: bool) -> bool {
    match option.map(str::trim) {
        Some("0") => true,
        Some("1") => false,
        _ => !float32_filtering,
    }
}

/// Feed one memory sample (what the app uses, what iOS still allows), in MiB.
///
/// `available_mib` is `None` when iOS gave no reading (unknown); `Some(0)` is a real
/// sample meaning no headroom left, and is recorded like any other.
pub fn note_sample(footprint_mib: u64, available_mib: Option<u64>) {
    PEAK_FOOTPRINT_MIB.fetch_max(footprint_mib, Ordering::Relaxed);
    if let Some(available_mib) = available_mib {
        MIN_AVAILABLE_MIB.fetch_min(available_mib, Ordering::Relaxed);
    }
}

/// Records why this run is ending (first reason wins).
pub fn set_exit_reason(reason: &str) {
    if let Ok(mut slot) = EXIT_REASON.lock()
        && slot.is_empty()
    {
        *slot = reason.to_owned();
    }
}

pub fn summary_line() -> String {
    let peak = PEAK_FOOTPRINT_MIB.load(Ordering::Relaxed);
    let min = MIN_AVAILABLE_MIB.load(Ordering::Relaxed);
    let min = if min == u64::MAX { "?".to_owned() } else { format!("{min} MiB") };
    let reason = EXIT_REASON
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default();
    let reason = if reason.is_empty() { "still running" } else { reason.as_str() };
    format!("session summary: peak footprint {peak} MiB | least memory iOS left {min} | {reason}")
}

/// Reads the previous run's log text and says how it ended. A clean exit prints
/// `lifecycle: process_exit`; any other end (user, iOS, crash) leaves no such line,
/// and the absence alone does not say which.
pub fn previous_run_note(previous_log: &str) -> String {
    if previous_log.trim().is_empty() {
        return "previous run: no log".to_owned();
    }
    if previous_log.contains("process_exit") {
        let code = previous_log
            .lines()
            .rev()
            .find(|line| line.contains("process_exit"))
            .and_then(|line| line.split("code=").nth(1))
            .map_or("?", str::trim);
        return format!("previous run: ended itself (exit code {code})");
    }
    let last_mem = previous_log
        .lines()
        .rev()
        .find(|line| line.starts_with("mem: footprint"))
        .map(|line| line.split(" | ").take(2).collect::<Vec<_>>().join(" | "))
        .unwrap_or_default();
    format!(
        "previous run: no exit line (closed by you, by iOS, or a crash; cause not proven). Last sample: {}",
        if last_mem.is_empty() { "none" } else { last_mem.as_str() }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadows_decision_follows_option_then_gpu() {
        assert!(shadows_decision(Some("0"), true));
        assert!(!shadows_decision(Some("1"), false));
        assert!(shadows_decision(None, false));
        assert!(!shadows_decision(None, true));
        assert!(shadows_decision(Some("auto"), false));
    }

    #[test]
    fn previous_run_clean_exit() {
        let text = "a\n[launch/info] lifecycle: process_exit pid=1 ns=2 code=1\n";
        assert_eq!(previous_run_note(text), "previous run: ended itself (exit code 1)");
    }

    #[test]
    fn previous_run_killed_names_last_sample() {
        let text = "x\nmem: footprint 2900 MiB | iOS still allows 90 MiB | heap live 1 MiB\n";
        let note = previous_run_note(text);
        assert!(note.contains("no exit line"));
        assert!(!note.contains("so iOS closed"), "absence of an exit is not proof of termination");
        assert!(note.contains("footprint 2900 MiB | iOS still allows 90 MiB"));
    }

    #[test]
    fn previous_run_missing_log() {
        assert_eq!(previous_run_note("  \n"), "previous run: no log");
    }

    #[test]
    fn summary_tracks_peak_and_minimum() {
        note_sample(100, Some(900));
        note_sample(300, Some(0));
        note_sample(200, Some(500));
        note_sample(250, None);
        let line = summary_line();
        assert!(line.contains("peak footprint"));
        assert!(line.contains("least memory iOS left"));
        assert!(line.contains("least memory iOS left 0 MiB"), "zero is a valid memory sample");
    }
}
